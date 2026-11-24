//! 设置相关命令。

use tauri::{AppHandle, Emitter, State};

use crate::domain::settings::Settings;
use crate::events;
use crate::service::tray;
use crate::state::SharedState;

#[tauri::command]
pub fn get_settings(state: State<'_, SharedState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, SharedState>,
    settings: Settings,
) -> Settings {
    let saved = state.replace_settings(settings);

    // 语言可能变了，托盘文案跟着刷新。
    let status = state.live_status();
    tray::refresh_labels(&app, saved.language, status.today_active_ms);
    let _ = app.emit(events::SETTINGS_CHANGED, &saved);

    // 设置改动可能影响"是否活跃"，立刻用新规则重算一次展示值。
    let refreshed = state.refresh_live_status(crate::domain::time::now_ms());
    let _ = app.emit(events::LIVE_STATUS, &refreshed);

    saved
}
