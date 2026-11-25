//! 闹钟调度：每 5 秒检查一次"当前分钟是否命中某个闹钟"。
//!
//! 命中条件与去重：
//! * 上班 / 下班闹钟按「星期 + 时分」精确匹配，同一分钟内只触发一次；
//! * 加班提醒只在下班之后、下一组闹钟上班之前有效，每隔 N 分钟触发一次，按分钟去重；
//! * 加班提醒一旦观察到"下班后工作过、又停下来"就当天结束，不再复活；
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
        let mut log = DayLog::default();
        loop {
            thread::sleep(CHECK_INTERVAL);
            check(&app, &state, &mut log);
        }
    });
}

/// 当天的调度记忆：已触发过的分钟、以及加班提醒的生命周期。跨天自动清空。
#[derive(Default)]
struct DayLog {
    date: Option<NaiveDate>,
    keys: HashSet<String>,
    /// 下班之后观察到过活跃的闹钟（"正在加班"）。
    overtime_active: HashSet<String>,
    /// 已经因为停止工作而结束加班提醒的闹钟。
    overtime_ended: HashSet<String>,
}

/// 加班状态按「闹钟 id + 下班时刻」记：用户在设置里改了下班时间，就等于换了一次加班。
fn overtime_key(alarm: &Alarm) -> String {
    format!("{}|{}", alarm.id, alarm.clock_out.minutes())
}

impl DayLog {
    fn roll(&mut self, today: NaiveDate) {
        if self.date != Some(today) {
            self.date = Some(today);
            self.keys.clear();
            self.overtime_active.clear();
            self.overtime_ended.clear();
        }
    }

    /// 当天第一次触发时返回 true。
    fn mark(&mut self, today: NaiveDate, key: String) -> bool {
        self.roll(today);
        self.keys.insert(key)
    }

    fn seen_active(&mut self, today: NaiveDate, alarm: &Alarm) {
        self.roll(today);
        self.overtime_active.insert(overtime_key(alarm));
    }

    /// 下班后确实工作过、现在停下来了 —— 这次加班到此为止。
    fn end_overtime(&mut self, today: NaiveDate, alarm: &Alarm) -> bool {
        self.roll(today);
        let key = overtime_key(alarm);
        if !self.overtime_active.contains(&key) {
            return false;
        }
        self.overtime_ended.insert(key)
    }

    fn overtime_ended(&self, alarm: &Alarm) -> bool {
        self.overtime_ended.contains(&overtime_key(alarm))
    }
}

fn check(app: &AppHandle, state: &SharedState, log: &mut DayLog) {
    let now = Local::now().naive_local();
    let today = now.date();
    // 跨天先把昨天的记忆清掉，避免早上误把昨天"已结束"的加班当成今天的状态。
    log.roll(today);
    let now_minutes = now.hour() as u32 * 60 + now.minute() as u32;
    let settings = state.settings();
    let live = state.live_status();
    let alarms = state.alarms();

    for alarm in &alarms {
        // 加班提醒的生命周期：过了下班时刻才算数。
        // 观察到活跃 → 记为"正在加班"；之后观察到空闲 → 这次加班结束，当天不再提醒。
        if alarm::active_on(alarm, today) && now_minutes >= alarm.clock_out.minutes() {
            if live.is_active {
                log.seen_active(today, alarm);
            } else if log.end_overtime(today, alarm) {
                eprintln!(
                    "[scheduler] 「{}」下班后已停止工作，结束当天的加班提醒",
                    alarm.label
                );
            }
        }

        if let Some(kind) = alarm::due_kind(alarm, now) {
            if should_skip(alarm, kind, &settings, live.is_active) {
                continue;
            }
            fire_once(app, state, log, alarm, kind, now, today);
            continue;
        }

        if settings.overtime_reminder
            && live.is_active
            && !log.overtime_ended(alarm)
            && alarm::overtime_due(alarm, &alarms, now, settings.overtime_interval_minutes)
        {
            fire_once(app, state, log, alarm, AlarmKind::Overtime, now, today);
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
    log: &mut DayLog,
    alarm: &Alarm,
    kind: AlarmKind,
    now: chrono::NaiveDateTime,
    today: NaiveDate,
) {
    let minute_of_day = now.hour() * 60 + now.minute();
    let key = format!("{}|{}|{}", alarm.id, kind.key(), minute_of_day);
    if !log.mark(today, key) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::alarm::ClockTime;

    fn alarm(id: &str, out: &str) -> Alarm {
        Alarm {
            id: id.into(),
            label: "工作".into(),
            clock_in: ClockTime::parse("09:00").unwrap(),
            clock_out: ClockTime::parse(out).unwrap(),
            ringtone_in: "a".into(),
            ringtone_out: "a".into(),
            weekdays: vec![true; 7],
            enabled: true,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 25).unwrap()
    }

    #[test]
    fn 下班后工作过再停下就结束当天提醒() {
        let a = alarm("a", "12:00");
        let mut log = DayLog::default();
        log.seen_active(today(), &a);
        assert!(!log.overtime_ended(&a));
        assert!(log.end_overtime(today(), &a));
        assert!(log.overtime_ended(&a));
        // 之后再检测到活跃也不该复活
        log.seen_active(today(), &a);
        assert!(log.overtime_ended(&a));
    }

    #[test]
    fn 没在下班后工作过不算结束() {
        let a = alarm("a", "12:00");
        let mut log = DayLog::default();
        // 刚启动就是空闲状态：无法证明"加班后停下来了"，保持待命
        assert!(!log.end_overtime(today(), &a));
        assert!(!log.overtime_ended(&a));
    }

    #[test]
    fn 跨天会清空加班状态() {
        let a = alarm("a", "12:00");
        let mut log = DayLog::default();
        log.seen_active(today(), &a);
        assert!(log.end_overtime(today(), &a));
        let tomorrow = today().succ_opt().unwrap();
        log.roll(tomorrow);
        assert!(!log.overtime_ended(&a));
    }

    #[test]
    fn 改了下班时间等于换一次加班() {
        let a = alarm("a", "12:00");
        let mut log = DayLog::default();
        log.seen_active(today(), &a);
        assert!(log.end_overtime(today(), &a));
        assert!(log.overtime_ended(&a));
        // 用户把下班时间改到 13:00，新的加班时段重新生效
        assert!(!log.overtime_ended(&alarm("a", "13:00")));
    }
}
