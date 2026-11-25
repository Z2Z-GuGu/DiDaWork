//! 闹钟模型：上下班成对设置，多组并存，时间段不可重叠。

use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::domain::time::weekday_index;

/// `HH:MM` 时刻。序列化为字符串，便于直接读配置文件。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClockTime {
    pub hour: u8,
    pub minute: u8,
}

impl ClockTime {
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self { hour, minute }
    }

    pub fn minutes(self) -> u32 {
        self.hour as u32 * 60 + self.minute as u32
    }

    pub fn parse(value: &str) -> Option<Self> {
        let (hour, minute) = value.trim().split_once(':')?;
        let hour: u8 = hour.parse().ok()?;
        let minute: u8 = minute.parse().ok()?;
        (hour < 24 && minute < 60).then_some(Self { hour, minute })
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl Serialize for ClockTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        ClockTime::parse(&raw).ok_or_else(|| de::Error::custom(format!("非法的时刻: {raw}")))
    }
}

/// 触发原因决定通知文案与铃声。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AlarmKind {
    ClockIn,
    ClockOut,
    /// 下班后仍在工作，按间隔重复提醒。
    Overtime,
}

impl AlarmKind {
    pub fn key(self) -> &'static str {
        match self {
            AlarmKind::ClockIn => "clockIn",
            AlarmKind::ClockOut => "clockOut",
            AlarmKind::Overtime => "overtime",
        }
    }
}

/// 一组上下班闹钟。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub id: String,
    /// 通知文案，例如「上班打卡」。
    pub label: String,
    pub clock_in: ClockTime,
    pub clock_out: ClockTime,
    pub ringtone_in: String,
    pub ringtone_out: String,
    /// 周一 → 周日。
    pub weekdays: Vec<bool>,
    pub enabled: bool,
}

impl Alarm {
    pub fn is_empty_label(&self) -> bool {
        self.label.trim().is_empty()
    }

    pub fn repeats_on(&self, weekday: usize) -> bool {
        self.weekdays.get(weekday).copied().unwrap_or(false)
    }

    pub fn repeats_any(&self) -> bool {
        self.weekdays.iter().any(|day| *day)
    }

    pub fn ringtone_for(&self, kind: AlarmKind) -> &str {
        match kind {
            AlarmKind::ClockIn => &self.ringtone_in,
            _ => &self.ringtone_out,
        }
    }

    fn range(&self) -> (u32, u32) {
        (self.clock_in.minutes(), self.clock_out.minutes())
    }

    /// 两组闹钟在时间轴上是否冲突（仅当启用且共享同一个星期时才算）。
    pub fn conflicts_with(&self, other: &Alarm) -> bool {
        if !self.enabled || !other.enabled {
            return false;
        }
        let share_day = (0..7).any(|day| self.repeats_on(day) && other.repeats_on(day));
        if !share_day {
            return false;
        }
        let (a_start, a_end) = self.range();
        let (b_start, b_end) = other.range();
        a_start < b_end && b_start < a_end
    }
}

/// 校验失败的原因。`code` 供前端做多语言文案，`message` 作为兜底。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
    pub conflicts: Vec<String>,
}

impl ValidationError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            conflicts: Vec::new(),
        }
    }
}

/// 校验一组闹钟。`others` 是除自己以外的所有闹钟。
pub fn validate(alarm: &Alarm, others: &[Alarm]) -> Result<(), ValidationError> {
    if alarm.is_empty_label() {
        return Err(ValidationError::new("labelRequired", "标签不能为空"));
    }
    if alarm.weekdays.len() != 7 {
        return Err(ValidationError::new("weekdaysInvalid", "重复设置不完整"));
    }
    if !alarm.repeats_any() {
        return Err(ValidationError::new("weekdaysRequired", "至少选择一天重复"));
    }
    if alarm.clock_out <= alarm.clock_in {
        return Err(ValidationError::new("rangeInvalid", "下班时间必须晚于上班时间"));
    }
    let conflicts: Vec<String> = others
        .iter()
        .filter(|other| other.id != alarm.id && alarm.conflicts_with(other))
        .map(|other| other.id.clone())
        .collect();
    if !conflicts.is_empty() {
        let mut error = ValidationError::new("overlap", "与已有闹钟时间段重叠");
        error.conflicts = conflicts;
        return Err(error);
    }
    Ok(())
}

/// 这个闹钟今天是否生效（已启用且今天在重复范围内）。
pub fn active_on(alarm: &Alarm, date: NaiveDate) -> bool {
    alarm.enabled && alarm.repeats_on(weekday_index(date))
}

/// 本分钟应当触发的上班/下班闹钟。
pub fn due_kind(alarm: &Alarm, now: NaiveDateTime) -> Option<AlarmKind> {
    if !active_on(alarm, now.date()) {
        return None;
    }
    let current = ClockTime::new(now.hour() as u8, now.minute() as u8);
    if current == alarm.clock_in {
        Some(AlarmKind::ClockIn)
    } else if current == alarm.clock_out {
        Some(AlarmKind::ClockOut)
    } else {
        None
    }
}

/// 这组闹钟的"加班时段"到几点结束（当天分钟数，默认 24:00）。
///
/// 加班提醒只服务于"下班了还在干"这个场景，所以它的有效区间是
/// **[下班时刻, 下一组闹钟的上班时刻)**——到了下午上班时间，就该由下午那组闹钟
/// 说话，上午的"已经下班，注意休息"不能再响。没有下一组闹钟时一直管到当天结束。
pub fn overtime_window_end(alarm: &Alarm, all: &[Alarm], date: NaiveDate) -> u32 {
    let out_minutes = alarm.clock_out.minutes();
    all.iter()
        .filter(|other| other.id != alarm.id && active_on(other, date))
        .map(|other| other.clock_in.minutes())
        .filter(|start| *start > out_minutes)
        .min()
        .unwrap_or(24 * 60)
}

/// 下班之后仍处于活跃状态时，是否到了下一次提醒时刻。
pub fn overtime_due(alarm: &Alarm, all: &[Alarm], now: NaiveDateTime, interval_minutes: u32) -> bool {
    if interval_minutes == 0 || !active_on(alarm, now.date()) {
        return false;
    }
    let now_minutes = now.hour() as u32 * 60 + now.minute() as u32;
    let out_minutes = alarm.clock_out.minutes();
    let end_minutes = overtime_window_end(alarm, all, now.date());
    now_minutes > out_minutes
        && now_minutes < end_minutes
        && (now_minutes - out_minutes) % interval_minutes == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alarm(id: &str, start: &str, end: &str) -> Alarm {
        Alarm {
            id: id.into(),
            label: "上班".into(),
            clock_in: ClockTime::parse(start).unwrap(),
            clock_out: ClockTime::parse(end).unwrap(),
            ringtone_in: "glass".into(),
            ringtone_out: "glass".into(),
            weekdays: vec![true; 7],
            enabled: true,
        }
    }

    #[test]
    fn 时刻序列化为零填充字符串() {
        let time = ClockTime::new(9, 5);
        assert_eq!(time.to_string(), "09:05");
        assert_eq!(ClockTime::parse("09:05"), Some(time));
        assert_eq!(ClockTime::parse("24:00"), None);
        assert_eq!(ClockTime::parse("9:5x"), None);
    }

    #[test]
    fn 重叠的闹钟会被拒绝() {
        let existing = vec![alarm("a", "09:00", "12:00")];
        let candidate = alarm("b", "11:00", "14:00");
        let error = validate(&candidate, &existing).unwrap_err();
        assert_eq!(error.code, "overlap");
        assert_eq!(error.conflicts, vec!["a".to_string()]);
    }

    #[test]
    fn 不相交的闹钟可以共存() {
        let existing = vec![alarm("a", "09:00", "12:00")];
        assert!(validate(&alarm("b", "13:00", "18:00"), &existing).is_ok());
        // 首尾相接不算重叠
        assert!(validate(&alarm("c", "12:00", "13:00"), &existing).is_ok());
    }

    #[test]
    fn 不同星期设置的闹钟不冲突() {
        let mut existing = alarm("a", "09:00", "12:00");
        existing.weekdays = vec![true, false, false, false, false, false, false];
        let mut candidate = alarm("b", "10:00", "11:00");
        candidate.weekdays = vec![false, true, false, false, false, false, false];
        assert!(validate(&candidate, std::slice::from_ref(&existing)).is_ok());
    }

    #[test]
    fn 下班时间必须晚于上班时间() {
        let error = validate(&alarm("b", "18:00", "09:00"), &[]).unwrap_err();
        assert_eq!(error.code, "rangeInvalid");
    }

    #[test]
    fn 下班后的加班提醒按间隔触发() {
        let alarm = alarm("a", "09:00", "18:00");
        let all = vec![alarm.clone()];
        let at = |hour, minute| {
            chrono::NaiveDate::from_ymd_opt(2026, 11, 24)
                .unwrap()
                .and_hms_opt(hour, minute, 0)
                .unwrap()
        };
        assert!(!overtime_due(&alarm, &all, at(18, 0), 10));
        assert!(!overtime_due(&alarm, &all, at(18, 5), 10));
        assert!(overtime_due(&alarm, &all, at(18, 10), 10));
        assert!(overtime_due(&alarm, &all, at(19, 0), 10));
        // 没有下一组闹钟时，加班提醒一直管到当天结束
        assert!(overtime_due(&alarm, &all, at(23, 50), 10));
    }

    #[test]
    fn 连续加班时提醒在一整天里落在该落的分钟() {
        let morning = alarm("m", "09:00", "12:00");
        let afternoon = alarm("a", "14:00", "18:00");
        let all = vec![morning.clone(), afternoon];
        let fired: Vec<String> = (12 * 60..14 * 60)
            .filter(|m| {
                let time = at((*m / 60) as u32, (*m % 60) as u32);
                overtime_due(&morning, &all, time, 10)
            })
            .map(|m| format!("{:02}:{:02}", m / 60, m % 60))
            .collect();
        // 12:10 起每 10 分钟一次，13:50 是最后一次；14:00 已经是下午上班时间。
        assert_eq!(fired.len(), 11);
        assert_eq!(fired.first().unwrap(), "12:10");
        assert_eq!(fired.last().unwrap(), "13:50");
    }

    fn at(hour: u32, minute: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, 24)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn 加班时段在下一组上班时间处截止() {
        let morning = alarm("m", "09:00", "12:00");
        let afternoon = alarm("a", "14:00", "18:00");
        let all = vec![morning.clone(), afternoon.clone()];
        assert_eq!(overtime_window_end(&morning, &all, at(13, 0).date()), 14 * 60);
        assert!(overtime_due(&morning, &all, at(13, 50), 10));
        // 14:00 已经是下午的上班时间，上午那组不能再提醒
        assert!(!overtime_due(&morning, &all, at(14, 0), 10));
        assert!(!overtime_due(&morning, &all, at(14, 10), 10));
        // 下午那组自己的加班时段从 18:00 开始
        assert_eq!(overtime_window_end(&afternoon, &all, at(20, 0).date()), 24 * 60);
        assert!(overtime_due(&afternoon, &all, at(18, 10), 10));
    }

    #[test]
    fn 停用或非当天的闹钟不会截断加班时段() {
        let morning = alarm("m", "09:00", "12:00");
        let mut paused = alarm("p", "14:00", "18:00");
        paused.enabled = false;
        let mut other_day = alarm("o", "15:00", "19:00");
        // 2026-11-24 是周二，只在周一重复的闹钟当天不生效
        other_day.weekdays = vec![true, false, false, false, false, false, false];
        let all = vec![morning.clone(), paused, other_day];
        assert_eq!(overtime_window_end(&morning, &all, at(13, 0).date()), 24 * 60);
        assert!(overtime_due(&morning, &all, at(14, 10), 10));
    }
}
