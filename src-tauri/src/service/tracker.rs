//! 采样服务：后台线程每分钟采集一次键鼠活动，落盘并广播状态。
//!
//! 为了兼顾"准点采样"与"低功耗"，线程每秒醒一次判断是否到点，
//! 真正干活的是每分钟一次的那次采样。

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::domain::activity::TrackerOutcome;
use crate::domain::time;
use crate::events;
use crate::platform::{self, InputMonitor};
use crate::service::tray;
use crate::state::SharedState;

/// 键鼠检测周期，产品要求固定 1 分钟。
pub const SAMPLE_INTERVAL_MS: i64 = 60_000;

/// 轮询粒度：只影响"到点"的精度，成本极低。
const TICK_INTERVAL: Duration = Duration::from_millis(1000);

/// 进行中的时段每隔这么久落盘一次。
/// 落盘会与已有区间合并（见 `coalesce`），因此反复写同一条不会产生重复数据，
/// 作用只是把"意外退出可能丢失的时长"限制在这个范围内。
const CHECKPOINT_INTERVAL_MS: i64 = 5 * 60_000;

pub fn spawn(app: AppHandle, state: SharedState) {
    thread::spawn(move || run(app, state));
}

fn run(app: AppHandle, state: SharedState) {
    let mut monitor: Box<dyn InputMonitor> = platform::input_monitor();
    // 启动时先给出一次状态，避免界面空白一分钟。
    state.refresh_live_status(time::now_ms());
    let mut next_tick = time::now_ms() + SAMPLE_INTERVAL_MS;
    let mut last_checkpoint = time::now_ms();

    loop {
        thread::sleep(TICK_INTERVAL);
        let now = time::now_ms();
        if now < next_tick {
            continue;
        }
        // 用固定步长推进，避免每秒循环带来的累计漂移。
        next_tick = now + SAMPLE_INTERVAL_MS;
        if now - last_checkpoint >= CHECKPOINT_INTERVAL_MS {
            // 只在真正活跃时写检查点，空闲时无需动磁盘。
            if let Some(segment) = state.with_tracker(|tracker| tracker.current_segment()) {
                state.commit_segment(segment);
            }
            last_checkpoint = now;
        }
        tick(&app, &state, monitor.as_mut(), now);
    }
}

fn tick(app: &AppHandle, state: &SharedState, monitor: &mut dyn InputMonitor, now: i64) {
    let settings = state.settings();
    let sample = monitor.sample();
    let working = sample.filtered(settings.detect_mouse, settings.detect_keyboard);

    // 1. 跨零点先切分，保证时段不会跨天存进同一个文件。
    let today = time::date_of(now);
    let midnight = time::start_of_day_ms(today);
    if let Some(head) = state.with_tracker(|tracker| tracker.roll_over(midnight)) {
        state.commit_segment(head);
    }

    // 2. 交给状态机判定。
    let outcome = state.with_tracker(|tracker| {
        tracker.observe(now, working, SAMPLE_INTERVAL_MS, settings.freeze_ms())
    });
    if let TrackerOutcome::Closed(segment) = outcome {
        state.commit_segment(segment);
    }

    // 3. 广播最新状态，供界面与托盘刷新。
    let status = state.refresh_live_status(now);
    let _ = app.emit(events::LIVE_STATUS, &status);
    tray::update_today(app, status.today_active_ms, settings.language);
}
