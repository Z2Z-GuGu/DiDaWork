//! DiDaWork —— 轻量的工作计时与上下班闹钟。
//!
//! 分层结构（自上而下单向依赖）：
//!
//! ```text
//!   commands   ← 前端调用的入口，只做参数整理与事件广播
//!      │
//!   service    ← 编排：采样线程、闹钟调度、响铃、托盘
//!      │
//!   domain     ← 纯逻辑：活跃状态机、闹钟规则、统计聚合（可单测，无 IO）
//!      │
//!   infra / platform  ← 外部世界：JSON 存储、系统键鼠/声音/通知能力
//! ```
//!
//! 所有业务规则都在 domain 里，替换存储或平台实现不影响上层。

mod commands;
mod domain;
mod events;
mod infra;
mod platform;
mod service;
mod state;

use std::sync::Arc;

use tauri::{Manager, RunEvent, WindowEvent};

use infra::store::Store;
use state::{AppState, SharedState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // macOS：只驻留菜单栏，不占 Dock。
            // 切换成 Accessory 后窗口偶尔会失去前台状态，这里显式再显示一次。
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                if let Some(window) = app.get_webview_window(service::window::MAIN_WINDOW) {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Windows / Linux 关掉原生标题栏（macOS 走透明的 overlay 标题栏），
            // 最小化 / 最大化 / 关闭由前端自绘，避免出现一条灰色标题条。
            #[cfg(not(target_os = "macos"))]
            if let Some(window) = app.get_webview_window(service::window::MAIN_WINDOW) {
                let _ = window.set_decorations(false);
            }

            let data_dir = app.path().app_data_dir()?;
            let store = Store::new(data_dir);
            store.ensure_layout()?;

            let state: SharedState = Arc::new(AppState::new(store));
            app.manage(Arc::clone(&state));

            service::tray::setup(app.handle(), state.settings().language)?;
            platform::notify::prepare(app.handle());
            service::tracker::spawn(app.handle().clone(), Arc::clone(&state));
            service::alarm_scheduler::spawn(app.handle().clone(), Arc::clone(&state));

            Ok(())
        })
        .on_window_event(|window, event| match event {
            // 主窗口关闭 = 收进菜单栏，而不是退出。
            WindowEvent::CloseRequested { api, .. }
                if window.label() == service::window::MAIN_WINDOW =>
            {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::alarms::list_alarms,
            commands::alarms::save_alarm,
            commands::alarms::delete_alarm,
            commands::alarms::set_alarm_enabled,
            commands::stats::get_live_status,
            commands::stats::get_day_summary,
            commands::stats::get_trend,
            commands::stats::get_timeline,
            commands::stats::get_heatmap,
            commands::app::get_app_info,
            commands::app::list_ringtones,
            commands::app::preview_ringtone,
            commands::app::dismiss_ring,
            commands::app::open_url,
            commands::app::open_data_dir,
            commands::app::open_main_window,
            commands::app::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("启动 DiDaWork 失败")
        .run(|app, event| {
            // 退出前把进行中的时段落盘，别丢掉最后几分钟的工作记录。
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<SharedState>() {
                    state.flush();
                }
            }
        });
}
