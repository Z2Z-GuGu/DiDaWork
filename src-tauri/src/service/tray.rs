//! 菜单栏图标（托盘）。
//!
//! macOS 下应用隐藏 Dock 图标，托盘是唯一入口：
//! 左键打开主界面，右键出菜单（含"今日已工作"，每分钟刷新；
//! 以及"关闭闹钟"，只在响铃期间可用——Windows 的通知没有按钮，这里是主要的停铃入口）。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::domain::settings::Language;
use crate::service::{ringer, window};
use crate::state::SharedState;

/// 需要动态改文案 / 可用状态的菜单项。
struct TrayItems {
    today: MenuItem<Wry>,
    dismiss: MenuItem<Wry>,
}

const TRAY_ID: &str = "didawork";

pub fn setup(app: &AppHandle, language: Language) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", label(language, "打开 DiDaWork", "Open DiDaWork"), true, None::<&str>)?;
    let today = MenuItem::with_id(app, "today", format_today(language, 0), false, None::<&str>)?;
    // 平时禁用：菜单里留一行灰掉的"关闭闹钟"，响铃时自动变亮。
    let dismiss = MenuItem::with_id(app, "dismiss", dismiss_label(language), false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", label(language, "退出", "Quit"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &today, &dismiss, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("DiDaWork")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => window::show_main(app),
            "dismiss" => {
                if let Some(state) = app.try_state::<SharedState>() {
                    ringer::dismiss(app, &state);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::toggle_main(tray.app_handle());
            }
        });

    builder = with_icon(builder, app);

    builder.build(app)?;
    app.manage(TrayItems { today, dismiss });
    Ok(())
}

/// 托盘 / 菜单栏图标单独用深色版：应用图标（Dock、Finder、通知横幅）保持不变。
///
/// 编译期嵌入，省掉运行时找资源目录；Tauri 会把它等比压到菜单栏的 18pt 高。
///
/// - macOS：`tray-mac.png` 是去掉白底、只剩黑色图形 + alpha 的版本，配 `icon_as_template`
///   交给系统着色（浅色菜单栏画黑、深色菜单栏画白），跟其他菜单栏图标一致。
/// - Windows / Linux：没有 template 概念，深色任务栏上纯黑图形会糊成一片，
///   所以仍用带白底圆角的 `tray.png`。
#[cfg(target_os = "macos")]
const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray-mac.png");
#[cfg(not(target_os = "macos"))]
const TRAY_ICON: &[u8] = include_bytes!("../../icons/tray.png");

fn with_icon(builder: TrayIconBuilder<Wry>, app: &AppHandle) -> TrayIconBuilder<Wry> {
    let builder = match Image::from_bytes(TRAY_ICON) {
        Ok(icon) => builder.icon(icon),
        // 解码失败也留个图标，退回应用图标总比空着好。
        Err(_) => match app.default_window_icon() {
            Some(icon) => builder.icon(icon.clone()),
            None => builder,
        },
    };
    // 只对 macOS 生效：让系统按菜单栏明暗自动反色，不去自己做深浅两套图。
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);
    builder
}

/// 每分钟调用一次，更新托盘菜单里的今日工作时长。
pub fn update_today(app: &AppHandle, active_ms: i64, language: Language) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.today.set_text(format_today(language, active_ms));
    }
}

/// 语言变化时刷新菜单文案。
pub fn refresh_labels(app: &AppHandle, language: Language, active_ms: i64) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.today.set_text(format_today(language, active_ms));
        let _ = items.dismiss.set_text(dismiss_label(language));
    }
}

/// 响铃开始 / 结束时由 `ringer` 调用，决定"关闭闹钟"是否可点。
pub fn set_ringing(app: &AppHandle, ringing: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.dismiss.set_enabled(ringing);
    }
}

fn dismiss_label(language: Language) -> String {
    label(language, "关闭闹钟", "Dismiss alarm")
}

fn label(language: Language, zh: &str, en: &str) -> String {
    match language {
        Language::En => en.to_string(),
        _ => zh.to_string(),
    }
}

fn format_today(language: Language, active_ms: i64) -> String {
    let minutes = active_ms / 60_000;
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (language, hours) {
        (Language::En, 0) => format!("Today: {rest}m"),
        (Language::En, _) => format!("Today: {hours}h {rest}m"),
        (_, 0) => format!("今日已工作 {rest} 分钟"),
        (_, _) => format!("今日已工作 {hours} 小时 {rest} 分"),
    }
}
