//! Minimal OpenHarmony host for a real GPUI `Render` view.
//!
//! The OHOS host passes its `OHNativeWindow` to these C ABI functions. A single
//! Rust render thread owns wgpu and the GPUI scene. Surface destruction waits
//! for that thread to release the renderer before ArkUI releases the window.

use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{c_char, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::{self, NonNull},
    rc::Rc,
    sync::{mpsc, OnceLock},
    time::{Duration, Instant},
};

use gpui::{
    size, App, Application, ApplicationHandle, DevicePixels, Pixels, PlatformInput, Point,
    RequestFrameOptions, TouchEvent, TouchId, TouchPhase, WindowVisibility,
};
use gpui_wgpu_ohos::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, OhosNdkWindowHandle,
    WindowHandle,
};

mod dispatcher;
mod platform;
mod window;

use crate::components::material::NavigationBarBuilder;
use crate::fling_guard::FlingGuard;
use crate::frame_pacer::FramePacer;
use platform::OhosPlatform;
use window::WindowState;

type AppCallback = fn(&mut App) -> Result<(), String>;
static APP_CALLBACK: OnceLock<AppCallback> = OnceLock::new();

/// Register the application root before the first XComponent surface arrives.
pub fn set_app_callback(callback: AppCallback) -> Result<(), &'static str> {
    APP_CALLBACK
        .set(callback)
        .map_err(|_| "OHOS app callback is already registered")
}

const BASE: u32 = 0x121318;
const TEXT: u32 = 0xE2E2E9;
const DEFAULT_DARK_MODE: bool = true;

#[no_mangle]
pub extern "C" fn gpui_ohos_background_color() -> u32 {
    display_rgb(BASE)
}

#[no_mangle]
pub extern "C" fn gpui_ohos_foreground_color() -> u32 {
    display_rgb(TEXT)
}

#[no_mangle]
pub extern "C" fn gpui_ohos_bottom_bar_color() -> u32 {
    display_rgb(NavigationBarBuilder::surface_color(DEFAULT_DARK_MODE))
}

fn display_rgb(color: u32) -> u32 {
    // GPUI's rgb channels reach the sRGB surface through linear-to-sRGB conversion.
    // Return the displayed color so ArkUI's system bars match the rendered pixels.
    fn display_channel(channel: u32) -> u32 {
        let linear = channel as f32 / 255.0;
        let srgb = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055 * linear.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round() as u32
    }

    (display_channel((color >> 16) & 0xff) << 16)
        | (display_channel((color >> 8) & 0xff) << 8)
        | display_channel(color & 0xff)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OhosWindow(usize);

// SAFETY: The OHOS host keeps this OHNativeWindow alive until
// `gpui_ohos_surface_destroyed` returns. The renderer only uses it on the
// dedicated render thread.
unsafe impl Send for OhosWindow {}
unsafe impl Sync for OhosWindow {}

impl HasWindowHandle for OhosWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let ptr = NonNull::new(self.0 as *mut c_void).ok_or(HandleError::Unavailable)?;
        let handle = OhosNdkWindowHandle::new(ptr);
        Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for OhosWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::ohos())
    }
}

enum Command {
    Surface {
        window: OhosWindow,
        width: u32,
        height: u32,
        scale: f32,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Touch {
        phase: u32,
        device_id: i64,
        native_id: i32,
        x: f32,
        y: f32,
    },
    Frame(bool),
    Task(gpui::RunnableVariant),
    Foreground(bool),
    Detach(mpsc::Sender<()>),
}

struct RenderState {
    gpu_context: GpuContext,
    window: Rc<RefCell<WindowState>>,
    application: Option<ApplicationHandle>,
    foreground: bool,
    touch_id: u64,
    active_touches: HashMap<(i64, i32), (TouchId, Point<Pixels>)>,
    fling_guard: FlingGuard,
}

impl RenderState {
    fn new() -> Self {
        Self {
            gpu_context: Rc::new(RefCell::new(None)),
            window: Rc::new(RefCell::new(WindowState::new())),
            application: None,
            foreground: false,
            touch_id: 0,
            active_touches: HashMap::new(),
            fling_guard: FlingGuard::new(),
        }
    }

    fn surface(
        &mut self,
        window: OhosWindow,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("OHOS surface has zero size".into());
        }
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        if self.window.borrow().window == Some(window) {
            let mut state = self.window.borrow_mut();
            state.width = width;
            state.height = height;
            state.scale = scale;
            if let Some(renderer) = &mut state.renderer {
                renderer.update_drawable_size(size(
                    DevicePixels(width as i32),
                    DevicePixels(height as i32),
                ));
            }
            let mut resize = state.resize.take();
            drop(state);
            if let Some(callback) = resize.as_mut() {
                callback(self.window.borrow().bounds().size, scale);
            }
            self.window.borrow_mut().resize = resize;
            self.frame(true);
            return Ok(());
        }

        if self.application.is_none() && APP_CALLBACK.get().is_none() {
            return Err("OHOS application root was not registered".into());
        }
        if self.window.borrow().window.is_some() {
            self.detach();
        }
        let config = WgpuSurfaceConfig {
            size: size(DevicePixels(width as i32), DevicePixels(height as i32)),
            transparent: false,
            preferred_present_mode: None,
        };
        {
            let mut state = self.window.borrow_mut();
            if let Some(renderer) = state.renderer.as_mut() {
                renderer
                    .recover(&window)
                    .map_err(|error| error.to_string())?;
                renderer.update_drawable_size(config.size);
            } else {
                state.renderer = Some(
                    WgpuRenderer::new(self.gpu_context.clone(), &window, config, None)
                        .map_err(|error| error.to_string())?,
                );
            }
            state.window = Some(window);
            state.width = width;
            state.height = height;
            state.scale = scale;
        }
        if self.application.is_none() {
            let callback = APP_CALLBACK.get().unwrap();
            let platform = Rc::new(
                OhosPlatform::new(self.window.clone(), sender().clone())
                    .map_err(|error| error.to_string())?,
            );
            let launch_error = Rc::new(RefCell::new(None));
            let report_error = launch_error.clone();
            let application =
                Application::with_platform(platform).run_embedded(move |cx: &mut App| {
                    if let Err(error) = callback(cx) {
                        *report_error.borrow_mut() = Some(error);
                    }
                });
            if let Some(error) = launch_error.borrow_mut().take() {
                self.destroy();
                return Err(error);
            }
            self.application = Some(application);
        } else {
            let mut resize = self.window.borrow_mut().resize.take();
            if let Some(callback) = resize.as_mut() {
                callback(self.window.borrow().bounds().size, scale);
            }
            self.window.borrow_mut().resize = resize;
        }
        self.update_active();
        self.frame(true);
        Ok(())
    }

    fn frame(&mut self, force_render: bool) {
        if !self.window.borrow().active {
            return;
        }
        let mut callback = self.window.borrow_mut().request_frame.take();
        if let Some(callback) = callback.as_mut() {
            callback(RequestFrameOptions {
                require_presentation: false,
                force_render,
            });
        }
        self.window.borrow_mut().request_frame = callback;
    }

    fn touch(&mut self, phase: u32, device_id: i64, native_id: i32, x: f32, y: f32) {
        if !self.window.borrow().active {
            return;
        }
        let phase = match phase {
            0 => TouchPhase::Started,
            1 => TouchPhase::Ended,
            2 => TouchPhase::Moved,
            3 => TouchPhase::Cancelled,
            _ => return,
        };
        let key = (device_id, native_id);
        let scale = self.window.borrow().scale;
        let position = gpui::point(gpui::px(x / scale), gpui::px(y / scale));
        let (id, replaced) = if phase == TouchPhase::Started {
            self.touch_id += 1;
            let id = TouchId(self.touch_id);
            (id, self.active_touches.insert(key, (id, position)))
        } else if let Some((id, last_position)) = self.active_touches.get_mut(&key) {
            *last_position = position;
            (*id, None)
        } else {
            return;
        };
        let event = TouchEvent {
            id,
            phase,
            position,
            predicted_position: None,
            force: None,
        };
        let mut callback = self.window.borrow_mut().input.take();
        if let Some(callback) = callback.as_mut() {
            if let Some((old_id, old_position)) = replaced {
                self.fling_guard.relay(
                    TouchEvent {
                        id: old_id,
                        phase: TouchPhase::Cancelled,
                        position: old_position,
                        predicted_position: None,
                        force: None,
                    },
                    || {
                        self.touch_id += 1;
                        TouchId(self.touch_id)
                    },
                    |event| {
                        callback(PlatformInput::Touch(event));
                    },
                );
            }
            self.fling_guard.relay(
                event,
                || {
                    self.touch_id += 1;
                    TouchId(self.touch_id)
                },
                |event| {
                    callback(PlatformInput::Touch(event));
                },
            );
        }
        self.window.borrow_mut().input = callback;
        if matches!(phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.active_touches.remove(&key);
        }
    }

    fn set_foreground(&mut self, foreground: bool) {
        if self.foreground == foreground {
            return;
        }
        self.foreground = foreground;
        self.update_active();
        if foreground {
            self.frame(true);
        }
    }

    fn update_active(&mut self) {
        let active = self.foreground && self.window.borrow().window.is_some();
        let (mut active_status, mut visibility_changed) = {
            let mut state = self.window.borrow_mut();
            if state.active == active {
                return;
            }
            state.active = active;
            (state.active_status.take(), state.visibility_changed.take())
        };
        if let Some(callback) = active_status.as_mut() {
            callback(active);
        }
        if let Some(callback) = visibility_changed.as_mut() {
            callback(if active {
                WindowVisibility::Visible
            } else {
                WindowVisibility::Hidden
            });
        }
        let mut state = self.window.borrow_mut();
        if state.active_status.is_none() {
            state.active_status = active_status;
        }
        if state.visibility_changed.is_none() {
            state.visibility_changed = visibility_changed;
        }
    }

    fn detach(&mut self) {
        self.active_touches.clear();
        self.fling_guard = FlingGuard::new();
        let mut state = self.window.borrow_mut();
        if let Some(renderer) = state.renderer.as_mut() {
            renderer.destroy();
        }
        state.window = None;
        state.frame_queued.set(false);
        drop(state);
        self.update_active();
    }

    fn destroy(&mut self) {
        self.detach();
        self.application = None;
        *self.window.borrow_mut() = WindowState::new();
        *self.gpu_context.borrow_mut() = None;
    }
}

fn sender() -> &'static mpsc::Sender<Command> {
    static SENDER: OnceLock<mpsc::Sender<Command>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("gpui-ohos-render".into())
            .spawn(move || {
                let mut state = RenderState::new();
                // GPUI can request the next animation frame through both
                // schedule_frame and frame_waker. Coalesce those requests and
                // pace the render thread instead of filling its command queue
                // with frames while a touch fling is active.
                const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
                let mut pacer = FramePacer::with_clock(FRAME_INTERVAL);
                loop {
                    let command = match pacer.poll_timeout(Instant::now()) {
                        Some(timeout) => match receiver.recv_timeout(timeout) {
                            Ok(command) => Some(command),
                            Err(mpsc::RecvTimeoutError::Timeout) => None,
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        },
                        None => match receiver.recv() {
                            Ok(command) => Some(command),
                            Err(_) => break,
                        },
                    };
                    if let Some(command) = command {
                        match command {
                            Command::Surface {
                                window,
                                width,
                                height,
                                scale,
                                reply,
                            } => {
                                let result = catch_unwind(AssertUnwindSafe(|| {
                                    state.surface(window, width, height, scale)
                                }));
                                match result {
                                    Ok(result) => {
                                        let _ = reply.send(result);
                                    }
                                    Err(payload) => {
                                        let message = payload
                                            .downcast_ref::<String>()
                                            .cloned()
                                            .or_else(|| {
                                                payload
                                                    .downcast_ref::<&str>()
                                                    .map(|s| s.to_string())
                                            })
                                            .unwrap_or_else(|| "non-string panic".to_string());
                                        let _ = reply
                                            .send(Err(format!("OHOS render panic: {message}")));
                                        break;
                                    }
                                }
                            }
                            Command::Touch {
                                phase,
                                device_id,
                                native_id,
                                x,
                                y,
                            } => state.touch(phase, device_id, native_id, x, y),
                            Command::Frame(force) => {
                                state.window.borrow().frame_queued.set(false);
                                if force {
                                    state.frame(true);
                                } else if state.window.borrow().active {
                                    pacer.schedule(Instant::now());
                                }
                            }
                            Command::Task(runnable) => {
                                runnable.run();
                            }
                            Command::Foreground(foreground) => {
                                state.set_foreground(foreground);
                                if !foreground {
                                    pacer = FramePacer::with_clock(FRAME_INTERVAL);
                                }
                            }
                            Command::Detach(reply) => {
                                state.detach();
                                pacer = FramePacer::with_clock(FRAME_INTERVAL);
                                let _ = reply.send(());
                            }
                        }
                    }
                    if pacer.take_frame(Instant::now()) {
                        state.frame(false);
                    }
                }
                state.destroy();
            })
            .expect("start OHOS render thread");
        sender
    })
}

/// Attach or resize the XComponent surface. The host must keep `window` valid
/// until `gpui_ohos_surface_destroyed` returns.
#[no_mangle]
pub extern "C" fn gpui_ohos_surface_created(
    window: *mut c_void,
    width: u32,
    height: u32,
    scale: f32,
    error_buffer: *mut c_char,
    error_capacity: usize,
) -> bool {
    let result = if window.is_null() || width == 0 || height == 0 {
        Err("OHOS surface has no window or has zero size".to_string())
    } else {
        let (reply, received) = mpsc::channel();
        if sender()
            .send(Command::Surface {
                window: OhosWindow(window as usize),
                width,
                height,
                scale,
                reply,
            })
            .is_err()
        {
            Err("OHOS render thread is unavailable".to_string())
        } else {
            received
                .recv()
                .unwrap_or_else(|_| Err("OHOS render thread exited during setup".to_string()))
        }
    };
    match result {
        Ok(()) => true,
        Err(error) => {
            if !error_buffer.is_null() && error_capacity > 0 {
                let length = error.len().min(error_capacity - 1);
                unsafe {
                    ptr::copy_nonoverlapping(error.as_ptr(), error_buffer.cast::<u8>(), length);
                    *error_buffer.add(length) = 0;
                }
            }
            false
        }
    }
}

/// Forward an OHOS touch phase and its XComponent-local physical coordinates.
#[no_mangle]
pub extern "C" fn gpui_ohos_touch(phase: u32, device_id: i64, native_id: i32, x: f32, y: f32) {
    let _ = sender().send(Command::Touch {
        phase,
        device_id,
        native_id,
        x,
        y,
    });
}

/// Forward UIAbility foreground/background changes to the GPUI render thread.
#[no_mangle]
pub extern "C" fn gpui_ohos_set_foreground(foreground: bool) {
    let _ = sender().send(Command::Foreground(foreground));
}

/// Synchronously release the GPU surface before ArkUI frees OHNativeWindow.
#[no_mangle]
pub extern "C" fn gpui_ohos_surface_destroyed() {
    let (reply, received) = mpsc::channel();
    if sender().send(Command::Detach(reply)).is_ok() {
        let _ = received.recv();
    }
}
