//! 系统通知（闹钟的提示通道）。
//!
//! 闹钟不再自建窗口，一切都交给系统通知中心：
//!
//! ```text
//!   alarm(title, body, action)  ──┬─ macOS   NSUserNotification + 主按钮
//!                                 ├─ Linux   freedesktop action 按钮
//!                                 └─ Windows WinRT toast（没有按钮）
//! ```
//!
//! 三端能力差异（诚实说明）：
//!
//! * **按钮**：macOS 的 `mainButton` 与 Linux 的 `actions` 都能带一个可点击按钮；
//!   Windows 的 toast 按钮必须自己拼 toast XML 并注册 AUMID，`tauri-plugin-notification`
//!   没有暴露这层能力，所以 Windows 上退化为纯文本通知，停铃靠托盘菜单或快捷键。
//! * **等待点击**：macOS / Linux 都是阻塞调用（通知被系统收起或用户点击时才返回），
//!   因此各自放在一条独立线程里跑；没有按钮就没有阻塞。
//!
//! 声音不在这里：铃声由 `platform::audio` 单独循环播放，通知保持静音，避免两路声音叠在一起。

#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::thread;

use tauri::AppHandle;

/// Linux 通知上按钮的 action id，回调里用它区分"点了按钮"和"通知被收起"。
#[cfg(target_os = "linux")]
const ACTION_ID: &str = "dismiss";

/// 一条闹钟通知。字段都是拥有所有权的 String，方便直接 move 进通知线程。
pub struct AlarmNotice {
    pub title: String,
    pub body: String,
    /// 按钮文案；`None` 表示这条通知不带按钮。
    pub action: Option<String>,
    /// 通知在屏幕上停留的时长（毫秒）。仅 Linux 使用，其余平台由系统决定。
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub ttl_ms: u32,
}

/// 弹出闹钟通知。用户按下通知上的按钮时调用 `on_action`。
///
/// 回调在通知线程里执行，所以要求 `Send + 'static`；它应该做成幂等的，
/// 因为回调可能晚于铃声自然结束才到达。
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub fn alarm(app: &AppHandle, notice: AlarmNotice, on_action: impl FnOnce() + Send + 'static) {
    #[cfg(target_os = "macos")]
    macos_alarm(app, notice, on_action);

    #[cfg(target_os = "linux")]
    linux_alarm(notice, on_action);

    #[cfg(target_os = "windows")]
    {
        // Windows 的 toast 没有按钮，这条通知不会被点击回调。
        let _ = on_action;
        windows_alarm(app, notice);
    }
}

/// 告诉通知中心"这条通知是哪个应用发的"，只做一次。
///
/// 不设的话，`send()` 会先跑一句 AppleScript `get id of application "use_default"`
/// 去猜身份——猜不到时 LaunchServices 就弹出一个「Where is use_default?」的
/// 选择应用窗口，每次响铃都挡在用户面前。所以发送前必须自己把身份定好：
///
/// * 在 `.app` 包里（正常安装后运行）用`tauri.conf.json` 里的 bundle id，
///   通知因此能带上应用名与图标；
/// * 裸二进制（`tauri dev` 跑的就是 `target/debug/didawork`）没有 bundle id，
///   借 Terminal 的身份，和 `tauri-plugin-notification` 的做法一致。
#[cfg(target_os = "macos")]
fn ensure_identity(app: &AppHandle) {
    use std::sync::Once;

    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let in_bundle = std::env::current_exe()
            .map(|path| path.to_string_lossy().contains(".app/Contents/MacOS/"))
            .unwrap_or(false);

        let bundle = if in_bundle {
            app.config().identifier.clone()
        } else {
            "com.apple.Terminal".to_string()
        };

        if let Err(err) = mac_notification_sys::set_application(&bundle) {
            // 设置失败也照发：库里的一次性开关已经用掉了，
            // 不会再回头去问系统"use_default 在哪"，那个窗口因此不会出现。
            eprintln!("[notify] 设置通知身份 {bundle} 失败: {err}");
        }
    });
}

/// 启动时先把应用身份定下来，第一条通知就不用现学。
///
/// 只有 macOS 需要；其它平台的通知库自己知道是谁发的。
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub fn prepare(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    ensure_identity(app);
}

/// macOS：`main_button` 负责那个可点击的按钮，`send()` 会一直阻塞到用户点击
/// 或系统把通知收起，所以整个流程放进独立线程。
#[cfg(target_os = "macos")]
fn macos_alarm(app: &AppHandle, notice: AlarmNotice, on_action: impl FnOnce() + Send + 'static) {
    use mac_notification_sys::{MainButton, Notification, NotificationResponse};

    ensure_identity(app);

    thread::spawn(move || {
        let mut notification = Notification::new();
        notification.title(&notice.title).message(&notice.body);
        if let Some(label) = notice.action.as_deref() {
            notification.main_button(MainButton::SingleAction(label));
        }
        match notification.send() {
            // 点按钮或点通知本体都算"我要停铃"。
            Ok(NotificationResponse::ActionButton(_)) | Ok(NotificationResponse::Click) => {
                on_action()
            }
            Ok(_) => {}
            Err(err) => eprintln!("[notify] 发送通知失败: {err}"),
        }
    });
}

/// Linux：freedesktop 通知规范里的 actions，GNOME / KDE 会把它渲染成按钮。
#[cfg(target_os = "linux")]
fn linux_alarm(notice: AlarmNotice, on_action: impl FnOnce() + Send + 'static) {
    use notify_rust::{Notification, Timeout};

    thread::spawn(move || {
        let mut notification = Notification::new();
        notification
            .appname("DiDaWork")
            .summary(&notice.title)
            .body(&notice.body)
            .timeout(Timeout::Milliseconds(notice.ttl_ms));
        if let Some(label) = notice.action.as_deref() {
            notification.action(ACTION_ID, label);
        }

        match notification.show() {
            // 只有带按钮的通知才需要守着；通知被收起时 action 是 "__closed"，直接忽略。
            Ok(handle) if notice.action.is_some() => handle.wait_for_action(|action| {
                if action == ACTION_ID {
                    on_action();
                }
            }),
            Ok(_) => {}
            Err(err) => eprintln!("[notify] 发送通知失败: {err}"),
        }
    });
}

/// Windows：走 `tauri-plugin-notification`（WinRT toast）。没有按钮，见模块文档。
#[cfg(target_os = "windows")]
fn windows_alarm(app: &AppHandle, notice: AlarmNotice) {
    use tauri_plugin_notification::NotificationExt;

    if let Err(err) = app
        .notification()
        .builder()
        .title(&notice.title)
        .body(&notice.body)
        .show()
    {
        eprintln!("[notify] 发送通知失败: {err}");
    }
}
