use std::{
    cell::RefCell,
    ffi::OsString,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{mpsc::Sender, Arc},
};

use anyhow::{anyhow, Result};
use futures::channel::oneshot;
use gpui::{
    Action, ActivityGuard, AnyWindowHandle, BackgroundExecutor, ClipboardItem, CursorStyle,
    DisplayId, DummyKeyboardMapper, ForegroundExecutor, Keymap, Menu, MenuItem, PathPromptOptions,
    Platform, PlatformDisplay, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTextSystem,
    PlatformWindow, Task, ThermalState, WindowAppearance, WindowParams,
};
use gpui_wgpu_ohos::CosmicTextSystem;

use super::{
    dispatcher::OhosDispatcher,
    window::{OhosPlatformWindow, WindowState},
    Command,
};

pub(super) struct OhosPlatform {
    state: Rc<RefCell<WindowState>>,
    sender: Sender<Command>,
    dispatcher: Arc<OhosDispatcher>,
    text: Arc<CosmicTextSystem>,
}

impl OhosPlatform {
    pub(super) fn new(state: Rc<RefCell<WindowState>>, sender: Sender<Command>) -> Result<Self> {
        let text = Arc::new(CosmicTextSystem::new_without_system_fonts("HarmonyOS Sans"));
        let mut fonts = Vec::new();
        for path in [
            "/system/fonts/HarmonyOS_Sans.ttf",
            "/system/fonts/HarmonyOS_Sans_SC.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.push(std::borrow::Cow::Owned(bytes));
            }
        }
        if fonts.is_empty() {
            return Err(anyhow!("HarmonyOS system fonts were not found"));
        }
        if let Ok(bytes) = std::fs::read("/system/fonts/HMOSColorEmojiCompat.ttf") {
            fonts.push(std::borrow::Cow::Owned(bytes));
        }
        text.add_fonts(fonts)?;
        Ok(Self {
            state,
            dispatcher: Arc::new(OhosDispatcher::new(sender.clone())),
            sender,
            text,
        })
    }
}

#[derive(Debug)]
struct OhosDisplay {
    bounds: gpui::Bounds<gpui::Pixels>,
}

impl PlatformDisplay for OhosDisplay {
    fn id(&self) -> DisplayId {
        DisplayId::new(1)
    }
    fn uuid(&self) -> Result<uuid::Uuid> {
        Ok(uuid::Uuid::nil())
    }
    fn bounds(&self) -> gpui::Bounds<gpui::Pixels> {
        self.bounds
    }
}

struct OhosKeyboardLayout;
impl PlatformKeyboardLayout for OhosKeyboardLayout {
    fn id(&self) -> &str {
        "ohos-default"
    }
    fn name(&self) -> &str {
        "OHOS"
    }
}

impl Platform for OhosPlatform {
    fn background_executor(&self) -> BackgroundExecutor {
        BackgroundExecutor::new(self.dispatcher.clone())
    }
    fn foreground_executor(&self) -> ForegroundExecutor {
        ForegroundExecutor::new(self.dispatcher.clone())
    }
    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.text.clone()
    }
    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        on_finish_launching();
    }
    fn quit(&self) {}
    fn restart(&self, _binary_path: Option<PathBuf>, _arguments: Vec<OsString>) {}
    fn activate(&self, _ignoring_other_apps: bool) {}
    fn hide(&self) {}
    fn hide_other_apps(&self) {}
    fn unhide_other_apps(&self) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![Rc::new(OhosDisplay {
            bounds: self.state.borrow().bounds(),
        })]
    }
    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        self.displays().into_iter().next()
    }
    fn active_window(&self) -> Option<AnyWindowHandle> {
        None
    }
    fn open_window(
        &self,
        _handle: AnyWindowHandle,
        _options: WindowParams,
    ) -> Result<Box<dyn PlatformWindow>> {
        Ok(Box::new(OhosPlatformWindow::new(
            self.state.clone(),
            self.sender.clone(),
        )))
    }
    fn window_appearance(&self) -> WindowAppearance {
        WindowAppearance::Dark
    }
    fn open_url(&self, _url: &str) {}
    fn on_open_urls(&self, _callback: Box<dyn FnMut(Vec<String>)>) {}
    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }
    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        _suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let (tx, rx) = oneshot::channel();
        let _ = tx.send(Ok(None));
        rx
    }
    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }
    fn reveal_path(&self, _path: &Path) {}
    fn open_with_system(&self, _path: &Path) {}
    fn on_quit(&self, _callback: Box<dyn FnMut() -> bool>) {}
    fn on_reopen(&self, _callback: Box<dyn FnMut()>) {}
    fn on_system_sleep(&self, _callback: Box<dyn FnMut()>) {}
    fn on_system_wake(&self, _callback: Box<dyn FnMut()>) {}
    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}
    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}
    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {}
    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {}
    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {}
    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }
    fn on_thermal_state_change(&self, _callback: Box<dyn FnMut()>) {}
    fn prevent_idle_sleep(&self, _reason: &str) -> Task<Result<ActivityGuard>> {
        Task::ready(Err(anyhow!("idle sleep control is unavailable on OHOS")))
    }
    fn app_path(&self) -> Result<PathBuf> {
        Ok(std::env::current_exe()?)
    }
    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        Err(anyhow!("auxiliary executables are unavailable on OHOS"))
    }
    fn set_cursor_style(&self, _style: CursorStyle) {}
    fn hide_cursor_until_mouse_moves(&self) {}
    fn is_cursor_visible(&self) -> bool {
        false
    }
    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }
    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        None
    }
    fn write_to_clipboard(&self, _item: ClipboardItem) {}
    fn read_from_primary(&self) -> Option<ClipboardItem> {
        None
    }
    fn write_to_primary(&self, _item: ClipboardItem) {}
    fn write_credentials(&self, _url: &str, _username: &str, _password: &[u8]) -> Task<Result<()>> {
        Task::ready(Err(anyhow!("credentials are unavailable on OHOS")))
    }
    fn read_credentials(&self, _url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Ok(None))
    }
    fn delete_credentials(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }
    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(OhosKeyboardLayout)
    }
    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }
    fn on_keyboard_layout_change(&self, _callback: Box<dyn FnMut()>) {}
}
