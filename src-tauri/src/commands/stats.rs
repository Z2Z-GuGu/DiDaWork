//! 统计相关命令。聚合全部在后端完成，前端拿到的就是可以直接画的数组。

use chrono::{Days, NaiveDate};
use tauri::State;

use crate::domain::activity::ActiveSegment;
use crate::domain::stats::{
    heatmap, summarize_day, trend, DaySummary, Heatmap, TimelineDay, Trend,
};
use crate::domain::time::{self, date_key, weekday_index};
use crate::domain::activity::coalesce;
use crate::state::{LiveStatus, SharedState};

#[tauri::command]
pub fn get_live_status(state: State<'_, SharedState>) -> LiveStatus {
    state.live_status()
}

/// 某天的详情；不传日期默认今天。今天的统计会包含"进行中"的时段。
#[tauri::command]
pub fn get_day_summary(state: State<'_, SharedState>, date: Option<String>) -> DaySummary {
    let day = date
        .as_deref()
        .and_then(time::parse_date)
        .unwrap_or_else(time::today);

    if day == time::today() {
        let (segments, live) = state.today_segments();
        DaySummary::from_segments(day, segments, live)
    } else {
        summarize_day(day, state.store.segments_on(day))
    }
}

/// 截止到 `end`（默认今天）的最近 `days` 天活跃时长（含今天进行中的部分）。
#[tauri::command]
pub fn get_trend(state: State<'_, SharedState>, days: u32, end: Option<String>) -> Trend {
    let days = days.clamp(1, 120) as u64;
    let to = resolve_end(end);
    let from = to - Days::new(days.saturating_sub(1));
    let points = state
        .store
        .segments_between(from, to)
        .into_iter()
        .map(|(date, segments)| (date, active_ms(state.inner(), date, &segments)))
        .collect();
    trend(points)
}

/// 截止到 `end`（默认今天）的最近 `days` 天时间段分布。
#[tauri::command]
pub fn get_timeline(state: State<'_, SharedState>, days: u32, end: Option<String>) -> Vec<TimelineDay> {
    let days = days.clamp(1, 31) as u64;
    let to = resolve_end(end);
    let from = to - Days::new(days.saturating_sub(1));
    state
        .store
        .segments_between(from, to)
        .into_iter()
        .map(|(date, mut segments)| {
            if date == time::today() {
                let (today_segments, _) = state.today_segments();
                segments = today_segments;
            }
            coalesce(&mut segments);
            TimelineDay {
                date: date_key(date),
                active_ms: segments.iter().map(ActiveSegment::duration_ms).sum(),
                segments,
            }
        })
        .collect()
}

/// 历史热力图，`weeks` 列，最后一列是本周。
#[tauri::command]
pub fn get_heatmap(state: State<'_, SharedState>, weeks: u32) -> Heatmap {
    let weeks = weeks.clamp(1, 53) as u64;
    let today = time::today();
    // 从 (weeks-1) 周前的周一开始，保证格子按整周对齐。
    let back = weekday_index(today) as u64 + (weeks - 1) * 7;
    let from = today - Days::new(back);
    let cells = state
        .store
        .segments_between(from, today)
        .into_iter()
        .map(|(date, segments)| (date, active_ms(state.inner(), date, &segments)))
        .collect();
    heatmap(cells)
}

fn resolve_end(end: Option<String>) -> NaiveDate {
    end.as_deref()
        .and_then(time::parse_date)
        .filter(|date| *date <= time::today())
        .unwrap_or_else(time::today)
}

/// 某天的活跃毫秒数；今天要算上进行中的时段。
fn active_ms(state: &SharedState, date: NaiveDate, stored: &[ActiveSegment]) -> i64 {
    if date == time::today() {
        let (segments, _) = state.today_segments();
        segments.iter().map(ActiveSegment::duration_ms).sum()
    } else {
        stored.iter().map(ActiveSegment::duration_ms).sum()
    }
}
