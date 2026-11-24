//! 应用级命令：铃声库、响铃关闭、窗口与退出。

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::domain::ringtone::Ringtone;
use crate::infra::ringtones::RingtoneError;
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

/// 扫描铃声目录：一个文件夹一首，文件夹名就是显示名。
#[tauri::command]
pub fn list_ringtones(state: State<'_, SharedState>) -> Vec<Ringtone> {
    state.ringtones().list()
}

/// 试听：播 5 秒，再点别的会打断上一首。
#[tauri::command]
pub fn preview_ringtone(id: String, state: State<'_, SharedState>) -> Result<(), String> {
    let Some(path) = state.ringtones().audio_path(&id) else {
        return Err("找不到这首铃声的音频文件".to_string());
    };
    audio::preview(&path);
    Ok(())
}

/// `add_ringtone` 的返回值：刷新后的列表 + 刚创建的那一首。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddedRingtone {
    pub ringtones: Vec<Ringtone>,
    pub created: Ringtone,
}

/// 导入用户自己的铃声：新建 `<数据目录>/ringtones/<名字>/`，复制音频与可选版权声明。
#[tauri::command]
pub fn add_ringtone(
    state: State<'_, SharedState>,
    name: String,
    audio_path: String,
    license_path: Option<String>,
) -> Result<AddedRingtone, RingtoneError> {
    let library = state.ringtones();
    let created = library.create(
        &name,
        Path::new(&audio_path),
        license_path.as_deref().map(Path::new),
    )?;
    Ok(AddedRingtone {
        ringtones: library.list(),
        created,
    })
}

/// 用默认应用打开某首铃声的版权声明。
#[tauri::command]
pub fn open_ringtone_license(id: String, state: State<'_, SharedState>) -> Result<(), String> {
    let Some(path) = state.ringtones().license_path(&id) else {
        return Err("这首铃声没有版权声明".to_string());
    };
    platform::open_external(&path.to_string_lossy()).map_err(|err| err.to_string())
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
