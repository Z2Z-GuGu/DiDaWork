//! 统计聚合：把"按天存储的时间段"整理成前端直接可画的形状。
//!
//! 所有聚合都在后端完成，前端只负责渲染——避免把逐日数据传到 WebView 再算一遍。

use chrono::NaiveDate;
use serde::Serialize;

use crate::domain::activity::ActiveSegment;
use crate::domain::time::{date_key, end_of_day_ms, start_of_day_ms, today};

/// 一天的详情。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub date: String,
    pub active_ms: i64,
    pub segments: Vec<ActiveSegment>,
    /// 当天最早的开始时间（毫秒时间戳），没有记录时为 None。
    pub first_start_ms: Option<i64>,
    /// 当天最晚的结束时间。
    pub last_end_ms: Option<i64>,
    /// 最长的一段连续活跃。
    pub longest_ms: i64,
    /// 该天是否包含"正在进行中"的时段。
    pub live: bool,
    /// 是否是今天。
    pub is_today: bool,
}

/// 近期趋势（每根柱子一天）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub date: String,
    pub active_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    pub points: Vec<TrendPoint>,
    pub total_ms: i64,
    /// 只对"有记录的天"求平均，避免把休息日拉低均值。
    pub average_ms: i64,
    pub max_ms: i64,
}

/// 某天的时间段分布（时间段曲线用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineDay {
    pub date: String,
    pub active_ms: i64,
    pub segments: Vec<ActiveSegment>,
}

/// 热力图格子。`level` 0..=4，0 表示没有任何活跃。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapCell {
    pub date: String,
    pub active_ms: i64,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Heatmap {
    pub cells: Vec<HeatmapCell>,
    pub max_ms: i64,
    /// 四档颜色对应的下界，供图例展示。
    pub thresholds: [i64; 4],
    /// 首个格子的星期偏移（周一=0），前端据此补空白。
    pub leading_blanks: u8,
}

impl DaySummary {
    pub fn from_segments(date: NaiveDate, mut segments: Vec<ActiveSegment>, live: bool) -> Self {
        segments.retain(|segment| !segment.is_empty());
        segments.sort_by_key(|segment| segment.start_ms);
        let active_ms = segments.iter().map(|segment| segment.duration_ms()).sum();
        DaySummary {
            date: date_key(date),
            active_ms,
            first_start_ms: segments.first().map(|segment| segment.start_ms),
            last_end_ms: segments.last().map(|segment| segment.end_ms),
            longest_ms: segments.iter().map(|segment| segment.duration_ms()).max().unwrap_or(0),
            segments,
            live,
            is_today: date == today(),
        }
    }
}

pub fn summarize_day(date: NaiveDate, segments: Vec<ActiveSegment>) -> DaySummary {
    DaySummary::from_segments(date, segments, false)
}

pub fn trend(points: Vec<(NaiveDate, i64)>) -> Trend {
    let total_ms = points.iter().map(|(_, ms)| *ms).sum::<i64>();
    let recorded: Vec<i64> = points.iter().map(|(_, ms)| *ms).filter(|ms| *ms > 0).collect();
    let average_ms = if recorded.is_empty() {
        0
    } else {
        recorded.iter().sum::<i64>() / recorded.len() as i64
    };
    Trend {
        max_ms: points.iter().map(|(_, ms)| *ms).max().unwrap_or(0),
        total_ms,
        average_ms,
        points: points
            .into_iter()
            .map(|(date, active_ms)| TrendPoint {
                date: date_key(date),
                active_ms,
            })
            .collect(),
    }
}

pub fn heatmap(cells: Vec<(NaiveDate, i64)>) -> Heatmap {
    let max_ms = cells.iter().map(|(_, ms)| *ms).max().unwrap_or(0);
    // 参考值取"实际最大值"与 4 小时中的较大者——空闲期不会让颜色全部变深。
    let reference = max_ms.max(4 * 3_600_000);
    let thresholds = [
        reference / 4,
        reference / 2,
        reference * 3 / 4,
        reference,
    ];
    let leading_blanks = cells
        .first()
        .map(|(date, _)| crate::domain::time::weekday_index(*date) as u8)
        .unwrap_or(0);
    Heatmap {
        max_ms,
        thresholds,
        leading_blanks,
        cells: cells
            .into_iter()
            .map(|(date, active_ms)| HeatmapCell {
                date: date_key(date),
                active_ms,
                level: level_of(active_ms, &thresholds),
            })
            .collect(),
    }
}

fn level_of(active_ms: i64, thresholds: &[i64; 4]) -> u8 {
    if active_ms <= 0 {
        return 0;
    }
    let mut level = 1;
    for (index, threshold) in thresholds.iter().enumerate() {
        if active_ms >= *threshold {
            level = (index + 1) as u8;
        }
    }
    level.min(4)
}

/// 一天的可用区间（用于把跨天时段裁剪到当天）。
pub fn day_bounds(date: NaiveDate) -> (i64, i64) {
    (start_of_day_ms(date), end_of_day_ms(date))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 3_600_000;

    #[test]
    fn 热力图分级在四档内() {
        let thresholds = [HOUR, 2 * HOUR, 3 * HOUR, 4 * HOUR];
        assert_eq!(level_of(0, &thresholds), 0);
        assert_eq!(level_of(HOUR / 2, &thresholds), 1);
        assert_eq!(level_of(HOUR, &thresholds), 1);
        assert_eq!(level_of(2 * HOUR, &thresholds), 2);
        assert_eq!(level_of(3 * HOUR, &thresholds), 3);
        assert_eq!(level_of(9 * HOUR, &thresholds), 4);
    }

    #[test]
    fn 平均值忽略没有记录的天() {
        let base = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let result = trend(vec![
            (base, 2 * HOUR),
            (base.succ_opt().unwrap(), 0),
            (base.succ_opt().unwrap().succ_opt().unwrap(), 4 * HOUR),
        ]);
        assert_eq!(result.average_ms, 3 * HOUR);
        assert_eq!(result.max_ms, 4 * HOUR);
        assert_eq!(result.total_ms, 6 * HOUR);
    }
}
