//! 本地时间小工具。
//!
//! 全项目统一使用「毫秒时间戳（i64）+ 本地日期」两个概念：
//! 时间戳用于记录精确区间，本地日期用于按天归档。

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, TimeZone};

/// 当前时间的毫秒时间戳。
pub fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

/// 时间戳 → 本地日期时间。
pub fn to_local(ms: i64) -> NaiveDateTime {
    match Local.timestamp_millis_opt(ms).single() {
        Some(dt) => dt.naive_local(),
        None => Local.timestamp_millis_opt(0).unwrap().naive_local(),
    }
}

/// 时间戳所属的本地日期。
pub fn date_of(ms: i64) -> NaiveDate {
    to_local(ms).date()
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// 某天 00:00:00 的时间戳。
pub fn start_of_day_ms(date: NaiveDate) -> i64 {
    let naive = date.and_hms_opt(0, 0, 0).expect("00:00:00 一定合法");
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|| naive.and_utc().timestamp_millis())
}

/// 某天 24:00 的时间戳（即次日 00:00）。
pub fn end_of_day_ms(date: NaiveDate) -> i64 {
    start_of_day_ms(date.succ_opt().unwrap_or(date))
}

pub fn date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn month_key(date: NaiveDate) -> String {
    date.format("%Y-%m").to_string()
}

pub fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

/// 周一 = 0，周日 = 6。
pub fn weekday_index(date: NaiveDate) -> usize {
    date.weekday().num_days_from_monday() as usize
}

/// 把天数换算成给人看的字符串，仅用于日志。
pub fn clock_label(ms: i64) -> String {
    to_local(ms).format("%H:%M").to_string()
}
