//! 应用级命令：声音试听、响铃关闭、窗口与退出。

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::domain::ringtone::{self, Ringtone};
use crate::platform::{self, audio};
use crate::service::{ringer, window};
use crate::state::SharedState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: String,
    pub data_dir: String,
    pub repository: String,
}

const REPOSITORY: &str = "https://github.com/bugu/DiDaWork";

#[tauri::command]
pub fn get_app_info(app: AppHandle, state: State<'_, SharedState>) -> AppInfo {
    AppInfo {
        version: app.package_info().version.to_string(),
        platform: platform::name().to_string(),
        data_dir: state.store.root().to_string_lossy().to_string(),
        repository: REPOSITORY.to_string(),
    }
}

#[tauri::command]
pub fn list_ringtones() -> Vec<Ringtone> {
    ringtone::all().to_vec()
}

#[tauri::command]
pub fn preview_ringtone(id: String) {
    audio::preview(ringtone::normalize(&id));
}

/// 关闭当前响铃（通知按钮、托盘菜单、主界面横幅、快捷键最终都走这里）。
#[tauri::command]
pub fn dismiss_ring(app: AppHandle, state: State<'_, SharedState>) {
    ringer::dismiss(&app, state.inner());
}

/// 在系统文件管理器里打开数据目录，方便用户备份/查看。
#[tauri::command]
pub fn open_data_dir(state: State<'_, SharedState>) -> Result<(), String> {
    let dir = state.store.root().to_string_lossy().to_string();
    platform::open_external(&dir).map_err(|err| err.to_string())
}

/// 用系统浏览器打开链接（设置页里的仓库地址）。
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    platform::open_external(&url).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn open_main_window(app: AppHandle) {
    window::show_main(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}
