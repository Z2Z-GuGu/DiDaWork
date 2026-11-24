//! 系统通知（闹钟的提示通道）。
//!
//! 闹钟不再自建窗口，一切都交给系统通知中心：
//!
//! ```text
//!   alarm(title, body, action)  ──┬─ macOS   UNUserNotificationCenter（带按钮）
//!                                 ├─ Linux   freedesktop action 按钮
//!                                 └─ Windows WinRT toast（没有按钮）
//! ```
//!
//! 三端能力差异（诚实说明）：
//!
//! * **按钮**：macOS 的通知分类（category）能带一个可点按钮，Linux 的 `actions` 同理；
//!   Windows 的 toast 按钮必须自己拼 toast XML 并注册 AUMID，`tauri-plugin-notification`
//!   没有暴露这层能力，所以 Windows 上退化为纯文本通知，停铃靠托盘菜单或快捷键。
//! * **弹不弹**：macOS 上应用处于前台时默认不弹横幅，而闹钟常常正好在用户盯着窗口时到点。
//!   `UNUserNotificationCenter` 的 `willPresent` 委托里显式返回「横幅 + 通知中心」即可强制弹出。
//! * **开发期**：`tauri dev` 跑的是 `target/debug/didawork` 裸二进制，没有 `.app`、没有
//!   bundle id，`UNUserNotificationCenter` 在这种情况下会直接抛异常。所以那时退回
//!   `mac-notification-sys`（借用 Terminal 的身份），横幅由系统自行决定。
//!
//! 声音不在这里：铃声由 `platform::audio` 单独循环播放，通知保持静音，避免两路声音叠在一起。

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

/// 启动时调用一次：装好通知代理、申请授权、定下应用身份。
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub fn prepare(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::prepare(app);
}

/// 弹出闹钟通知。用户按下通知上的按钮（或点通知本体）时调用 `on_action`。
///
/// 回调可能晚于响铃结束才到达，所以调用方要保证它是幂等的。
#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub fn alarm(app: &AppHandle, notice: AlarmNotice, on_action: impl FnOnce() + Send + 'static) {
    #[cfg(target_os = "macos")]
    macos::alarm(notice, Box::new(on_action));

    #[cfg(target_os = "linux")]
    linux_alarm(notice, on_action);

    #[cfg(target_os = "windows")]
    {
        // Windows 的 toast 没有按钮，这条通知不会被点击回调。
        let _ = on_action;
        windows_alarm(app, notice);
    }
}

/// 响铃结束（按钮、快捷键、托盘、横幅、到点自动收尾）时把通知一并收掉，
/// 免得通知中心里留一条已经没意义的提醒。
pub fn clear() {
    #[cfg(target_os = "macos")]
    macos::clear();
}

/// macOS：走 `UserNotifications` 框架。
///
/// 早先用的是 `NSUserNotification`，它在 macOS 11 起被标记弃用，现在的系统上交付
/// 变得很不可靠——同一份代码会出现"这次弹、下次不弹"，所以换成现在的官方接口：
///
/// * `prepare` 申请授权（系统会弹一次"允许通知"的询问）并挂上委托；
/// * `willPresent` 里返回 `Banner | List`，应用在前台也照弹；
/// * 按钮由通知分类（category）里的 `UNNotificationAction` 提供。
#[cfg(target_os = "macos")]
mod macos {
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2::{define_class, AllocAnyThread};
    use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSSet, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
        UNNotificationAction, UNNotificationActionOptions, UNNotificationCategory,
        UNNotificationCategoryOptions, UNNotificationPresentationOptions, UNNotificationRequest,
        UNNotificationResponse, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };
    use tauri::AppHandle;

    use super::AlarmNotice;

    /// 「关闭闹钟」按钮的标识，回调里靠它区分"点了按钮"和"点了通知本体"。
    const DISMISS_ACTION: &str = "didawork.dismiss";
    /// 通知分类标识：同一类通知共用一组按钮。
    const CATEGORY: &str = "didawork.alarm";
    /// 点通知本体时系统给的动作标识。
    const DEFAULT_ACTION: &str = "com.apple.UNNotificationDefaultActionIdentifier";

    /// 一次响铃的"待点击"回调。同一时刻只会响一个闹钟，一个槽位就够。
    struct Pending {
        id: String,
        on_action: Box<dyn FnOnce() + Send>,
    }

    static PENDING: Mutex<Option<Pending>> = Mutex::new(None);
    /// 当前挂在通知中心里的那条通知，响铃结束时按 id 收掉。
    static CURRENT: Mutex<Option<String>> = Mutex::new(None);
    /// `false` = 没有 `.app` 的裸二进制，用不了 `UserNotifications`。
    static MODERN: OnceLock<bool> = OnceLock::new();

    define_class!(
        // 通知中心的委托。`setDelegate:` 是弱引用，所以要自己泄漏一份保活（见 `delegate()`）。
        #[unsafe(super(NSObject))]
        #[name = "DiDaWorkNotificationDelegate"]
        #[ivars = ()]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                completion_handler: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                // 不实现这个方法时，应用在前台会被当作"不用打扰"，横幅直接不发。
                // 闹钟是用户主动设的，任何时候都该弹出来。
                completion_handler.call((
                    UNNotificationPresentationOptions::Banner
                        | UNNotificationPresentationOptions::List,
                ));
            }

            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                completion_handler: &block2::DynBlock<dyn Fn()>,
            ) {
                completion_handler.call(());

                // 点按钮、点通知本体都算"我要停铃"。
                let action = response.actionIdentifier().to_string();
                if action != DISMISS_ACTION && action != DEFAULT_ACTION {
                    return;
                }
                let identifier = response.notification().request().identifier().to_string();

                let pending = {
                    let mut slot = PENDING.lock().unwrap_or_else(|err| err.into_inner());
                    let is_current = slot
                        .as_ref()
                        .is_some_and(|pending| pending.id == identifier);
                    // 只有"发这条通知的那一次响铃"还在等回调时才执行，
                    // 这样点旧通知不会误停后来那一次铃声。
                    if is_current {
                        slot.take().map(|pending| pending.on_action)
                    } else {
                        None
                    }
                };
                if let Some(action) = pending {
                    action();
                }
            }
        }
    );

    /// 全局唯一的委托对象。
    fn delegate() -> &'static Delegate {
        static ADDR: OnceLock<usize> = OnceLock::new();
        let addr = *ADDR.get_or_init(|| {
            let object = Delegate::alloc().set_ivars(());
            let object: Retained<Delegate> = unsafe { objc2::msg_send![super(object), init] };
            // 泄漏：这个对象要跟着进程活到最后，由系统直接调用。
            Retained::into_raw(object) as usize
        });
        // SAFETY: 地址来自上面泄漏的对象，进程生命周期内不会失效。
        unsafe { &*(addr as *const Delegate) }
    }

    fn center() -> Retained<UNUserNotificationCenter> {
        UNUserNotificationCenter::currentNotificationCenter()
    }

    /// 当前进程是否跑在 `.app` 里。
    ///
    /// 不能只看 `tauri::is_dev()`：debug 打包出来的 `.app` 同样是 bundle 运行，只是没优化。
    fn in_bundle() -> bool {
        std::env::current_exe()
            .map(|path| path.to_string_lossy().contains(".app/Contents/MacOS/"))
            .unwrap_or(false)
    }

    fn modern() -> bool {
        *MODERN.get().unwrap_or(&false)
    }

    pub fn prepare(_app: &AppHandle) {
        let modern = in_bundle();
        let _ = MODERN.set(modern);

        if !modern {
            // 裸二进制：没有 bundle id，`UNUserNotificationCenter` 会抛异常。
            // 借 Terminal 的身份发送（与 tauri 通知插件的开发期做法一致）。
            eprintln!("[notify] 开发模式：没有 bundle id，通知走降级通道（前台可能不弹）");
            if let Err(err) = mac_notification_sys::set_application("com.apple.Terminal") {
                eprintln!("[notify] 设置通知身份失败: {err}");
            }
            return;
        }

        let center = center();
        let proto = ProtocolObject::from_ref(delegate());
        center.setDelegate(Some(proto));

        // 首次运行系统会问"是否允许 DiDaWork 发送通知"，之后不再问。
        // 闹钟是核心功能，不允许的话通知就发不出来，所以启动时就要这一次。
        let handler = RcBlock::new(
            |granted: objc2::runtime::Bool, _error: *mut objc2_foundation::NSError| {
                if granted.as_bool() {
                    eprintln!("[notify] 通知已授权");
                } else {
                    eprintln!("[notify] 通知未获授权，闹钟将只有铃声");
                }
            },
        );
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &handler,
        );
    }

    /// 注册通知分类，按钮文案跟着语言走，所以每次响铃前用当前文案刷一遍。
    fn install_category(label: &str) {
        let action = UNNotificationAction::actionWithIdentifier_title_options(
            &NSString::from_str(DISMISS_ACTION),
            &NSString::from_str(label),
            UNNotificationActionOptions::empty(),
        );
        let actions = NSArray::from_retained_slice(&[action]);
        let category = UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
            &NSString::from_str(CATEGORY),
            &actions,
            &NSArray::new(),
            UNNotificationCategoryOptions::empty(),
        );
        center().setNotificationCategories(&NSSet::from_retained_slice(&[category]));
    }

    pub fn alarm(notice: AlarmNotice, on_action: Box<dyn FnOnce() + Send>) {
        if !modern() {
            legacy_alarm(notice, on_action);
            return;
        }

        let id = format!("didawork-{}", now_ms());
        *PENDING.lock().unwrap_or_else(|err| err.into_inner()) = Some(Pending {
            id: id.clone(),
            on_action,
        });
        *CURRENT.lock().unwrap_or_else(|err| err.into_inner()) = Some(id.clone());

        if let Some(label) = notice.action.as_deref() {
            install_category(label);
        }

        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notice.title));
        content.setBody(&NSString::from_str(&notice.body));
        content.setCategoryIdentifier(&NSString::from_str(CATEGORY));
        // 声音交给 platform::audio 循环播放，通知本身保持静音，避免两路声音叠在一起。

        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&id),
            &content,
            None,
        );
        let delivery = RcBlock::new(|error: *mut objc2_foundation::NSError| {
            if !error.is_null() {
                let error = unsafe { &*error };
                eprintln!("[notify] 投递失败: {error}");
            }
        });
        center().addNotificationRequest_withCompletionHandler(&request, Some(&delivery));
    }

    pub fn clear() {
        let Some(id) = CURRENT
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .take()
        else {
            return;
        };

        {
            let mut slot = PENDING.lock().unwrap_or_else(|err| err.into_inner());
            if slot.as_ref().is_some_and(|pending| pending.id == id) {
                *slot = None;
            }
        }

        if modern() {
            center().removeDeliveredNotificationsWithIdentifiers(&NSArray::from_retained_slice(&[
                NSString::from_str(&id),
            ]));
        }
    }

    /// 开发期（没有 `.app`）的降级通道：`mac-notification-sys`。
    ///
    /// 它没有 `willPresent` 那层委托，应用在前台时系统可能不弹横幅——这是开发期的
    /// 已知差异，打包后的应用走上面的 `UserNotifications`，不受影响。
    fn legacy_alarm(notice: AlarmNotice, on_action: Box<dyn FnOnce() + Send>) {
        use mac_notification_sys::{MainButton, Notification, NotificationResponse};
        use std::thread;

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

    fn now_ms() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or_default()
    }
}

/// Linux：freedesktop 通知规范里的 actions，GNOME / KDE 会把它渲染成按钮。
#[cfg(target_os = "linux")]
fn linux_alarm(notice: AlarmNotice, on_action: impl FnOnce() + Send + 'static) {
    use notify_rust::{Notification, Timeout};
    use std::thread;

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
