//! 主窗口的显示 / 隐藏。窗口关闭时只是隐藏，应用继续在菜单栏驻留。

use tauri::{AppHandle, Manager, WebviewWindow};

pub const MAIN_WINDOW: &str = "main";

pub fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW)
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn toggle_main(app: &AppHandle) {
    if let Some(window) = main_window(app) {
        let visible = window.is_visible().unwrap_or(false);
        let focused = window.is_focused().unwrap_or(false);
        if visible && focused {
            let _ = window.hide();
        } else {
            show_main(app);
        }
    }
}
