use std::{cell::RefCell, ffi::c_void, ptr::NonNull, rc::Rc, sync::Arc};

use futures::channel::oneshot;
use gpui::{
    self, Capslock, DispatchEventResult, GpuSpecs, Modifiers, PlatformAtlas, PlatformDisplay,
    PlatformInput, PlatformInputHandler, PlatformWindow, PromptButton, PromptLevel,
    RequestFrameOptions, WindowAppearance, WindowBackgroundAppearance, WindowBounds,
    WindowControlArea, WindowVisibility,
};
use gpui_wgpu_ohos::WgpuRenderer;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, OhosNdkWindowHandle,
    WindowHandle,
};

use super::{Command, OhosWindow};

pub(super) struct WindowState {
    pub(super) window: Option<OhosWindow>,
    pub(super) renderer: Option<WgpuRenderer>,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale: f32,
    pub(super) request_frame: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    pub(super) input: Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    pub(super) resize: Option<Box<dyn FnMut(gpui::Size<gpui::Pixels>, f32)>>,
    input_handler: Option<PlatformInputHandler>,
}

impl WindowState {
    pub(super) fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            width: 0,
            height: 0,
            scale: 1.0,
            request_frame: None,
            input: None,
            resize: None,
            input_handler: None,
        }
    }

    pub(super) fn bounds(&self) -> gpui::Bounds<gpui::Pixels> {
        gpui::Bounds {
            origin: gpui::point(gpui::px(0.0), gpui::px(0.0)),
            size: gpui::size(
                gpui::px(self.width as f32 / self.scale),
                gpui::px(self.height as f32 / self.scale),
            ),
        }
    }
}

pub(super) struct OhosPlatformWindow {
    state: Rc<RefCell<WindowState>>,
    sender: std::sync::mpsc::Sender<Command>,
}

impl OhosPlatformWindow {
    pub(super) fn new(
        state: Rc<RefCell<WindowState>>,
        sender: std::sync::mpsc::Sender<Command>,
    ) -> Self {
        Self { state, sender }
    }
}

impl HasWindowHandle for OhosPlatformWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let ptr = self
            .state
            .borrow()
            .window
            .and_then(|window| NonNull::new(window.0 as *mut c_void))
            .ok_or(HandleError::Unavailable)?;
        let handle = OhosNdkWindowHandle::new(ptr);
        Ok(unsafe { WindowHandle::borrow_raw(handle.into()) })
    }
}

impl HasDisplayHandle for OhosPlatformWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::ohos())
    }
}

impl PlatformWindow for OhosPlatformWindow {
    fn bounds(&self) -> gpui::Bounds<gpui::Pixels> {
        self.state.borrow().bounds()
    }

    fn is_maximized(&self) -> bool {
        true
    }
    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Fullscreen(self.bounds())
    }
    fn content_size(&self) -> gpui::Size<gpui::Pixels> {
        self.bounds().size
    }
    fn resize(&mut self, _size: gpui::Size<gpui::Pixels>) {}
    fn scale_factor(&self) -> f32 {
        self.state.borrow().scale
    }
    fn appearance(&self) -> WindowAppearance {
        WindowAppearance::Dark
    }
    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        None
    }
    fn mouse_position(&self) -> gpui::Point<gpui::Pixels> {
        gpui::Point::default()
    }
    fn modifiers(&self) -> Modifiers {
        Modifiers::default()
    }
    fn capslock(&self) -> Capslock {
        Capslock::default()
    }

    fn set_input_handler(&mut self, handler: PlatformInputHandler) {
        self.state.borrow_mut().input_handler = Some(handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.state.borrow_mut().input_handler.take()
    }

    fn prompt(
        &self,
        _level: PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<oneshot::Receiver<usize>> {
        None
    }
    fn activate(&self) {}
    fn is_active(&self) -> bool {
        self.state.borrow().renderer.is_some()
    }
    fn visibility(&self) -> WindowVisibility {
        WindowVisibility::Visible
    }
    fn is_hovered(&self) -> bool {
        false
    }
    fn background_appearance(&self) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Opaque
    }
    fn set_title(&mut self, _title: &str) {}
    fn set_background_appearance(&self, _appearance: WindowBackgroundAppearance) {}
    fn minimize(&self) {}
    fn zoom(&self) {}
    fn toggle_fullscreen(&self) {}
    fn is_fullscreen(&self) -> bool {
        true
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        self.state.borrow_mut().request_frame = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        self.state.borrow_mut().input = Some(callback);
    }

    fn on_active_status_change(&self, _callback: Box<dyn FnMut(bool)>) {}
    fn on_visibility_change(&self, _callback: Box<dyn FnMut(WindowVisibility)>) {}
    fn on_hover_status_change(&self, _callback: Box<dyn FnMut(bool)>) {}
    fn on_resize(&self, callback: Box<dyn FnMut(gpui::Size<gpui::Pixels>, f32)>) {
        self.state.borrow_mut().resize = Some(callback);
    }
    fn on_moved(&self, _callback: Box<dyn FnMut()>) {}
    fn on_should_close(&self, _callback: Box<dyn FnMut() -> bool>) {}
    fn on_hit_test_window_control(&self, _callback: Box<dyn FnMut() -> Option<WindowControlArea>>) {
    }
    fn on_close(&self, _callback: Box<dyn FnOnce()>) {}
    fn on_appearance_changed(&self, _callback: Box<dyn FnMut()>) {}

    fn frame_waker(&self) -> Option<Rc<dyn Fn()>> {
        let sender = self.sender.clone();
        Some(Rc::new(move || {
            let _ = sender.send(Command::Frame(false));
        }))
    }

    fn schedule_frame(&self) {
        let _ = self.sender.send(Command::Frame(false));
    }

    fn draw(&self, scene: &gpui::Scene) {
        if let Some(renderer) = self.state.borrow_mut().renderer.as_mut() {
            renderer.draw(scene);
        }
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.state
            .borrow()
            .renderer
            .as_ref()
            .expect("OHOS renderer")
            .sprite_atlas()
            .clone()
    }

    fn is_subpixel_rendering_supported(&self) -> bool {
        false
    }
    fn gpu_specs(&self) -> Option<GpuSpecs> {
        self.state
            .borrow()
            .renderer
            .as_ref()
            .map(WgpuRenderer::gpu_specs)
    }
    fn update_ime_position(&self, _bounds: gpui::Bounds<gpui::Pixels>) {}
}
