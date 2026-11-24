//! 响铃编排：声音 + 系统通知 + 快捷键关闭。
//!
//! 生命周期：
//!
//! ```text
//!   ring()  ─┬─ 循环播放铃声（最长 N 秒）
//!            ├─ 弹出系统通知（macOS / Linux 带"关闭闹钟"按钮）
//!            ├─ 起一个守护线程轮询"双 Shift"  ── 按下 → dismiss()
//!            └─ 起一个定时线程，N 秒后自动收尾
//!
//!   dismiss() ─┬─ 停声、清空会话
//!              └─ 通知按钮 / 托盘菜单 / 快捷键 / 主界面横幅都走这里（幂等）
//! ```
//!
//! 不再创建任何自绘窗口：提示统一交给系统通知中心，用户在哪都能看见、都能关掉。
//! 会话本身由 `state.ring_slot()` 持有，天然保证"同一时刻只响一个闹钟"。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::domain::alarm::AlarmKind;
use crate::domain::settings::{Language, Settings};
use crate::domain::time;
use crate::events;
use crate::platform::audio::SoundHandle;
use crate::platform::notify::{self, AlarmNotice};
use crate::platform::{self};
use crate::state::SharedState;

const RING_POLL_INTERVAL: Duration = Duration::from_millis(80);

/// 前端展示"正在响铃"横幅需要的全部信息。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RingInfo {
    pub alarm_id: String,
    pub kind: AlarmKind,
    pub label: String,
    pub ringtone: String,
    pub started_ms: i64,
    pub clock: String,
}

pub struct RingSession {
    /// 本次响铃的起点，用来分辨"这条通知是哪一次响铃"。
    started_ms: i64,
    watcher_stop: Arc<AtomicBool>,
    sound: Option<SoundHandle>,
}

impl RingSession {
    fn silence(&mut self) {
        self.watcher_stop.store(true, Ordering::Relaxed);
        if let Some(mut sound) = self.sound.take() {
            sound.stop();
        }
    }
}

/// 开始（或替换）一次响铃。
pub fn ring(app: &AppHandle, state: &SharedState, info: RingInfo) {
    // 上一个还没关掉就直接覆盖，避免多路声音叠在一起。
    dismiss(app, state);

    let settings = state.settings();
    // 铃声是磁盘上的音乐文件；配置里那首被删掉了就用列表第一首，总比没声音强。
    let audio = state
        .ringtones()
        .audio_path(&info.ringtone)
        .or_else(|| state.ringtones().first_audio());
    if audio.is_none() {
        eprintln!("[ringer] 没有可用的铃声文件，只弹通知");
    }
    let session = RingSession {
        started_ms: info.started_ms,
        watcher_stop: Arc::new(AtomicBool::new(false)),
        sound: audio.map(|path| SoundHandle::play_file(&path, settings.ring_max_seconds as u64)),
    };
    let watcher_stop = Arc::clone(&session.watcher_stop);
    *state.ring_slot() = Some(session);

    // 托盘应用没有控制台，留一行日志方便排查"到点没响"这类问题。
    eprintln!("[ringer] 响铃 {} · {}", info.label, info.clock);
    let _ = app.emit(events::RING_START, &info);
    crate::service::tray::set_ringing(app, true);
    notify_alarm(app, state, &info, &settings);
    spawn_dismiss_watcher(app.clone(), state.clone(), watcher_stop, settings.dismiss_shortcut);
    // 铃声到点会自己停，会话和界面也要跟着收尾，否则"正在响铃"会一直挂着。
    spawn_auto_stop(app.clone(), state.clone(), info.started_ms, settings.ring_max_seconds);
}

/// 结束响铃（幂等）：通知按钮晚到、重复触发都不会有问题。
pub fn dismiss(app: &AppHandle, state: &SharedState) {
    let session = { state.ring_slot().take() };
    let Some(mut session) = session else {
        return;
    };
    session.silence();
    // 铃声没了，通知也别继续挂在通知中心。
    notify::clear();
    eprintln!("[ringer] 停铃");
    let _ = app.emit(events::RING_END, ());
    crate::service::tray::set_ringing(app, false);
}

/// 只关闭"发这条通知的那一次"响铃。
///
/// 通知会一直留在通知中心，用户可能过很久才点它的按钮；如果那时新的闹钟正在响，
/// 点旧通知不该把新铃声掐掉。
pub fn dismiss_session(app: &AppHandle, state: &SharedState, started_ms: i64) {
    let is_current = {
        let slot = state.ring_slot();
        slot.as_ref().map(|session| session.started_ms) == Some(started_ms)
    };
    if is_current {
        dismiss(app, state);
    }
}

/// 到点自动收尾：铃声停下的同时清掉会话、横幅与托盘状态。
fn spawn_auto_stop(app: AppHandle, state: SharedState, started_ms: i64, seconds: u32) {
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(seconds as u64));
        dismiss_session(&app, &state, started_ms);
    });
}

/// 守护线程：只在响铃期间存在，每 80ms 读一次修饰键状态，几乎不占 CPU。
fn spawn_dismiss_watcher(
    app: AppHandle,
    state: SharedState,
    stop: Arc<AtomicBool>,
    shortcut: crate::domain::settings::Shortcut,
) {
    thread::spawn(move || {
        let mut watcher = platform::shortcut_watcher();
        while !stop.load(Ordering::Relaxed) {
            if watcher.pressed(shortcut) {
                // 这里不 join 自己，直接调用是安全的：dismiss 只置位并停声。
                dismiss(&app, &state);
                return;
            }
            thread::sleep(RING_POLL_INTERVAL);
        }
    });
}

/// 弹系统通知：标题用闹钟名，正文是"该做什么 + 触发时刻"，按钮用来停铃。
fn notify_alarm(app: &AppHandle, state: &SharedState, info: &RingInfo, settings: &Settings) {
    let notice = AlarmNotice {
        title: info.label.clone(),
        body: format!("{} · {}", message(settings.language, info.kind), info.clock),
        action: Some(action_label(settings.language).to_string()),
        // 通知至少停留 10 秒，别在铃声还没响完时先消失。
        ttl_ms: settings.ring_max_seconds.max(10) * 1_000,
    };

    let handle = app.clone();
    let shared = Arc::clone(state);
    let started_ms = info.started_ms;
    notify::alarm(app, notice, move || {
        dismiss_session(&handle, &shared, started_ms)
    });
}

fn message(language: Language, kind: AlarmKind) -> &'static str {
    match (language, kind) {
        (Language::En, AlarmKind::ClockIn) => "Time to start working",
        (Language::En, AlarmKind::ClockOut) => "Time to call it a day",
        (Language::En, AlarmKind::Overtime) => "You are still working — take a break",
        (_, AlarmKind::ClockIn) => "该上班了",
        (_, AlarmKind::ClockOut) => "该下班了",
        (_, AlarmKind::Overtime) => "已经下班，注意休息",
    }
}

fn action_label(language: Language) -> &'static str {
    match language {
        Language::En => "Dismiss alarm",
        _ => "关闭闹钟",
    }
}

/// 供调度器构造响铃信息。
pub fn build_info(alarm_id: &str, kind: AlarmKind, label: &str, ringtone: &str) -> RingInfo {
    let now = time::now_ms();
    RingInfo {
        alarm_id: alarm_id.to_string(),
        kind,
        label: label.to_string(),
        ringtone: ringtone.to_string(),
        started_ms: now,
        clock: time::clock_label(now),
    }
}
