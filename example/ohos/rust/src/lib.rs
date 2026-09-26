//! OHOS example root. The platform owns the surface and calls this app's view callback.

use gpui::{App, AppContext, WindowOptions};

mod view;

fn open_root(cx: &mut App) -> Result<(), String> {
    cx.open_window(WindowOptions::default(), |_, cx| {
        cx.new(|_| view::Router::new())
    })
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[no_mangle]
pub extern "C" fn gpui_ohos_register_app() -> bool {
    gpui_mobile::ohos::set_app_callback(open_root).is_ok()
}
