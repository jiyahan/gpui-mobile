//! Minimal OpenHarmony host for a real GPUI `Render` view.
//!
//! The OHOS host passes its `OHNativeWindow` to these C ABI functions. A single
//! Rust render thread owns wgpu and the GPUI scene. Surface destruction waits
//! for that thread to release the renderer before ArkUI releases the window.

use std::{
    cell::RefCell,
    ffi::{c_char, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::{self, NonNull},
    rc::Rc,
    sync::{mpsc, OnceLock},
};

use gpui::{
    size, App, AppContext, Application, ApplicationHandle, DevicePixels, PlatformInput,
    RequestFrameOptions, TouchEvent, TouchId, TouchPhase, WindowOptions,
};
use gpui_wgpu_ohos::{GpuContext, WgpuRenderer, WgpuSurfaceConfig};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, OhosNdkWindowHandle,
    WindowHandle,
};

mod dispatcher;
mod platform;
mod view;
mod window;

use crate::components::material::NavigationBarBuilder;
use platform::OhosPlatform;
use view::Router;
use window::WindowState;

#[no_mangle]
pub extern "C" fn gpui_ohos_background_color() -> u32 {
    display_rgb(view::BASE)
}

#[no_mangle]
pub extern "C" fn gpui_ohos_foreground_color() -> u32 {
    display_rgb(view::TEXT)
}

#[no_mangle]
pub extern "C" fn gpui_ohos_bottom_bar_color() -> u32 {
    display_rgb(NavigationBarBuilder::surface_color(view::DEFAULT_DARK_MODE))
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
        x: f32,
        y: f32,
    },
    Frame(bool),
    Task(gpui::RunnableVariant),
    Destroy(mpsc::Sender<()>),
}

struct RenderState {
    gpu_context: GpuContext,
    window: Rc<RefCell<WindowState>>,
    application: Option<ApplicationHandle>,
    touch_id: u64,
}

impl RenderState {
    fn new() -> Self {
        Self {
            gpu_context: Rc::new(RefCell::new(None)),
            window: Rc::new(RefCell::new(WindowState::new())),
            application: None,
            touch_id: 0,
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

        self.destroy();
        let config = WgpuSurfaceConfig {
            size: size(DevicePixels(width as i32), DevicePixels(height as i32)),
            transparent: false,
            preferred_present_mode: None,
        };
        let renderer = WgpuRenderer::new(self.gpu_context.clone(), &window, config, None)
            .map_err(|error| error.to_string())?;
        {
            let mut state = self.window.borrow_mut();
            state.window = Some(window);
            state.width = width;
            state.height = height;
            state.scale = scale;
            state.renderer = Some(renderer);
        }
        let platform = Rc::new(
            OhosPlatform::new(self.window.clone(), sender().clone())
                .map_err(|error| error.to_string())?,
        );
        let launch_error = Rc::new(RefCell::new(None));
        let report_error = launch_error.clone();
        let application = Application::with_platform(platform).run_embedded(move |cx: &mut App| {
            if let Err(error) =
                cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Router::new()))
            {
                *report_error.borrow_mut() = Some(error.to_string());
            }
        });
        if let Some(error) = launch_error.borrow_mut().take() {
            self.destroy();
            return Err(error);
        }
        self.application = Some(application);
        self.frame(true);
        Ok(())
    }

    fn frame(&mut self, force_render: bool) {
        let mut callback = self.window.borrow_mut().request_frame.take();
        if let Some(callback) = callback.as_mut() {
            callback(RequestFrameOptions {
                require_presentation: false,
                force_render,
            });
        }
        self.window.borrow_mut().request_frame = callback;
    }

    fn touch(&mut self, phase: u32, x: f32, y: f32) {
        let phase = match phase {
            0 => {
                self.touch_id += 1;
                TouchPhase::Started
            }
            1 => TouchPhase::Ended,
            2 => TouchPhase::Moved,
            3 => TouchPhase::Cancelled,
            _ => return,
        };
        let scale = self.window.borrow().scale;
        let event = PlatformInput::Touch(TouchEvent {
            id: TouchId(self.touch_id),
            phase,
            position: gpui::point(gpui::px(x / scale), gpui::px(y / scale)),
            predicted_position: None,
            force: None,
        });
        let mut callback = self.window.borrow_mut().input.take();
        if let Some(callback) = callback.as_mut() {
            callback(event);
        }
        self.window.borrow_mut().input = callback;
    }

    fn destroy(&mut self) {
        self.application = None;
        if let Some(mut renderer) = self.window.borrow_mut().renderer.take() {
            renderer.destroy();
        }
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
                for command in receiver {
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
                                            payload.downcast_ref::<&str>().map(|s| s.to_string())
                                        })
                                        .unwrap_or_else(|| "non-string panic".to_string());
                                    let _ =
                                        reply.send(Err(format!("OHOS render panic: {message}")));
                                    break;
                                }
                            }
                        }
                        Command::Touch { phase, x, y } => state.touch(phase, x, y),
                        Command::Frame(force) => state.frame(force),
                        Command::Task(runnable) => {
                            runnable.run();
                        }
                        Command::Destroy(reply) => {
                            state.destroy();
                            let _ = reply.send(());
                        }
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
pub extern "C" fn gpui_ohos_touch(phase: u32, x: f32, y: f32) {
    let _ = sender().send(Command::Touch { phase, x, y });
}

/// Synchronously release the GPU surface before ArkUI frees OHNativeWindow.
#[no_mangle]
pub extern "C" fn gpui_ohos_surface_destroyed() {
    let (reply, received) = mpsc::channel();
    if sender().send(Command::Destroy(reply)).is_ok() {
        let _ = received.recv();
    }
}
