//! 闹钟调度：每 5 秒检查一次"当前分钟是否命中某个闹钟"。
//!
//! 命中条件与去重：
//! * 上班 / 下班闹钟按「星期 + 时分」精确匹配，同一分钟内只触发一次；
//! * 加班提醒在下班之后每隔 N 分钟触发一次，同样按分钟去重；
//! * 跳过规则在这里统一处理（已在工作中 / 本来就没在工作）。

use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use chrono::{Local, NaiveDate, Timelike};
use tauri::AppHandle;

use crate::domain::alarm::{self, Alarm, AlarmKind};
use crate::service::ringer;
use crate::state::SharedState;

const CHECK_INTERVAL: Duration = Duration::from_secs(5);

pub fn spawn(app: AppHandle, state: SharedState) {
    thread::spawn(move || {
        let mut fired = FiredLog::default();
        loop {
            thread::sleep(CHECK_INTERVAL);
            check(&app, &state, &mut fired);
        }
    });
}

/// 已触发记录，按天重置。
#[derive(Default)]
struct FiredLog {
    date: Option<NaiveDate>,
    keys: HashSet<String>,
}

impl FiredLog {
    /// 当天第一次触发时返回 true。
    fn mark(&mut self, today: NaiveDate, key: String) -> bool {
        if self.date != Some(today) {
            self.date = Some(today);
            self.keys.clear();
        }
        self.keys.insert(key)
    }
}

fn check(app: &AppHandle, state: &SharedState, fired: &mut FiredLog) {
    let now = Local::now().naive_local();
    let today = now.date();
    let settings = state.settings();
    let live = state.live_status();
    let alarms = state.alarms();

    for alarm in &alarms {
        if let Some(kind) = alarm::due_kind(alarm, now) {
            if should_skip(alarm, kind, &settings, live.is_active) {
                continue;
            }
            fire_once(app, state, fired, alarm, kind, now, today);
            continue;
        }

        if settings.overtime_reminder
            && live.is_active
            && alarm::overtime_due(alarm, now, settings.overtime_interval_minutes)
        {
            fire_once(app, state, fired, alarm, AlarmKind::Overtime, now, today);
        }
    }
}

/// 产品规则：上班时已在工作就不打扰，下班时本来就没工作也不打扰。
fn should_skip(
    _alarm: &Alarm,
    kind: AlarmKind,
    settings: &crate::domain::settings::Settings,
    is_active: bool,
) -> bool {
    match kind {
        AlarmKind::ClockIn => settings.skip_clock_in_when_active && is_active,
        AlarmKind::ClockOut => settings.skip_clock_out_when_idle && !is_active,
        AlarmKind::Overtime => false,
    }
}

fn fire_once(
    app: &AppHandle,
    state: &SharedState,
    fired: &mut FiredLog,
    alarm: &Alarm,
    kind: AlarmKind,
    now: chrono::NaiveDateTime,
    today: NaiveDate,
) {
    let minute_of_day = now.hour() * 60 + now.minute();
    let key = format!("{}|{}|{}", alarm.id, kind.key(), minute_of_day);
    if !fired.mark(today, key) {
        return;
    }
    let info = ringer::build_info(
        &alarm.id,
        kind,
        &alarm.label,
        alarm.ringtone_for(kind),
    );
    ringer::ring(app, state, info);
}
