//! 活跃状态机。
//!
//! 采样周期固定为 1 分钟，每次采样只能得到"上一分钟内有没有键鼠输入"这一个布尔值。
//! 状态机负责把稀疏的采样点还原成「活跃时间段」：
//!
//! ```text
//! 采样:   ●  ●  ●  ●  ○  ○  ○  ○  ○  ●  ●
//! 输入:  有  有  有  有  无  无  无  无  无  有  有
//! 状态: 活跃 ─────────────┐
//!            （冻结窗口）  │ 超过冻结阈值才判定为离开
//! 记录:  [───────────────]              [───
//! ```
//!
//! 关键规则（来自产品设计）：
//! 1. 静止不超过「冻结时间」时不结束时段，重新活跃即视为持续活跃；
//! 2. 判定离开的时间点是**最后一次检测到键鼠输入**的时间，而不是判定时刻。

use serde::{Deserialize, Serialize};

/// 一段连续的活跃时间，半开区间 `[start_ms, end_ms)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSegment {
    pub start_ms: i64,
    pub end_ms: i64,
}

impl ActiveSegment {
    pub fn new(start_ms: i64, end_ms: i64) -> Self {
        Self { start_ms, end_ms }
    }

    pub fn duration_ms(&self) -> i64 {
        (self.end_ms - self.start_ms).max(0)
    }

    pub fn is_empty(&self) -> bool {
        self.duration_ms() <= 0
    }

    pub fn merge(&mut self, other: &ActiveSegment) {
        self.end_ms = self.end_ms.max(other.end_ms);
    }

    /// 裁剪到 `[from, to)`，完全落在外面时返回 `None`。
    pub fn clamp(&self, from: i64, to: i64) -> Option<ActiveSegment> {
        let start = self.start_ms.max(from);
        let end = self.end_ms.min(to);
        (end > start).then_some(ActiveSegment { start_ms: start, end_ms: end })
    }
}

/// 把一组时段排序、合并相邻碎片。落盘前调用，保证文件里的数据始终"干净"。
pub fn coalesce(segments: &mut Vec<ActiveSegment>) {
    /// 小于这个空隙的两段视为同一段，用来抵消采样边界抖动。
    const MERGE_GAP_MS: i64 = 60_000;

    segments.retain(|segment| !segment.is_empty());
    segments.sort_by_key(|segment| segment.start_ms);
    let mut merged: Vec<ActiveSegment> = Vec::with_capacity(segments.len());
    for segment in segments.drain(..) {
        match merged.last_mut() {
            Some(last) if segment.start_ms - last.end_ms <= MERGE_GAP_MS => last.merge(&segment),
            _ => merged.push(segment),
        }
    }
    *segments = merged;
}

/// 一次采样后状态机给出的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackerOutcome {
    /// 空闲，没有变化。
    Idle,
    /// 活跃中，没有变化（时段内部延展）。
    Active,
    /// 新时段开始，需要通知调用方。
    Started,
    /// 时段结束，调用方应当把它持久化。
    Closed(ActiveSegment),
}

/// 活跃状态机。只保存"当前这一段"，历史时段由调用方落盘。
#[derive(Debug, Default)]
pub struct ActivityTracker {
    current: Option<ActiveSegment>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self { current: None }
    }

    /// 当前时段的起点。
    pub fn current_start_ms(&self) -> Option<i64> {
        self.current.map(|seg| seg.start_ms)
    }

    /// 把当前时段按原样取出（不结束状态），用于"含进行中时段"的统计。
    pub fn current_segment(&self) -> Option<ActiveSegment> {
        self.current
    }

    /// 喂一次采样。
    ///
    /// * `now_ms` —— 本次采样时刻
    /// * `has_input` —— 上一个采样周期内是否检测到键鼠输入
    /// * `interval_ms` —— 采样周期
    /// * `freeze_ms` —— 允许的最长静止时间
    pub fn observe(
        &mut self,
        now_ms: i64,
        has_input: bool,
        interval_ms: i64,
        freeze_ms: i64,
    ) -> TrackerOutcome {
        match self.current {
            None => {
                if has_input {
                    // 输入发生在 (now - interval, now] 之间，保守地从上一个采样点算起。
                    self.current = Some(ActiveSegment::new(now_ms - interval_ms, now_ms));
                    TrackerOutcome::Started
                } else {
                    TrackerOutcome::Idle
                }
            }
            Some(mut segment) => {
                if has_input {
                    segment.end_ms = now_ms;
                    self.current = Some(segment);
                    TrackerOutcome::Active
                } else if now_ms - segment.end_ms >= freeze_ms {
                    // 超过冻结阈值：在"最后一次检测到输入"的时刻结束。
                    self.current = None;
                    TrackerOutcome::Closed(segment)
                } else {
                    // 仍在冻结窗口内，继续算作活跃。
                    TrackerOutcome::Active
                }
            }
        }
    }

    /// 强制结束当前时段（退出、跨天、暂停统计时使用）。
    pub fn force_close(&mut self) -> Option<ActiveSegment> {
        self.current.take().filter(|seg| !seg.is_empty())
    }

    /// 跨天切分：把 0 点之前的部分吐出来，当前时段从 0 点重新开始。
    pub fn roll_over(&mut self, midnight_ms: i64) -> Option<ActiveSegment> {
        let segment = self.current?;
        if segment.start_ms >= midnight_ms {
            return None;
        }
        self.current = Some(ActiveSegment::new(midnight_ms, segment.end_ms.max(midnight_ms)));
        Some(ActiveSegment::new(segment.start_ms, midnight_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn 单次输入开启时段且从上一采样点起算() {
        let mut tracker = ActivityTracker::new();
        let outcome = tracker.observe(10 * MIN, true, MIN, 5 * MIN);
        assert_eq!(outcome, TrackerOutcome::Started);
        assert_eq!(tracker.current_start_ms(), Some(9 * MIN));
    }

    #[test]
    fn 冻结窗口内的短暂静止不断开时段() {
        let mut tracker = ActivityTracker::new();
        tracker.observe(10 * MIN, true, MIN, 5 * MIN);
        // 静止 3 分钟，小于冻结阈值
        for minute in 11..=13 {
            let outcome = tracker.observe(minute * MIN, false, MIN, 5 * MIN);
            assert_eq!(outcome, TrackerOutcome::Active);
        }
        // 重新活跃 → 依然是同一段
        tracker.observe(14 * MIN, true, MIN, 5 * MIN);
        let segment = tracker.current_segment().unwrap();
        assert_eq!(segment.start_ms, 9 * MIN);
        assert_eq!(segment.end_ms, 14 * MIN);
    }

    #[test]
    fn 超过冻结阈值时在最后一次输入处结束() {
        let mut tracker = ActivityTracker::new();
        tracker.observe(10 * MIN, true, MIN, 5 * MIN);
        let mut closed = None;
        for minute in 11..=16 {
            if let TrackerOutcome::Closed(segment) = tracker.observe(minute * MIN, false, MIN, 5 * MIN)
            {
                closed = Some(segment);
            }
        }
        // 最后一次输入在 10min，冻结 5min，11..16 分钟无输入 → 15min 时关闭
        assert_eq!(closed, Some(ActiveSegment::new(9 * MIN, 10 * MIN)));
        assert!(tracker.current_segment().is_none());
    }

    #[test]
    fn 跨零点会切分成两段() {
        let mut tracker = ActivityTracker::new();
        tracker.observe(10 * MIN, true, MIN, 5 * MIN);
        let midnight = 30 * MIN;
        let head = tracker.roll_over(midnight).unwrap();
        assert_eq!(head, ActiveSegment::new(9 * MIN, midnight));
        assert_eq!(tracker.current_start_ms(), Some(midnight));
    }

    #[test]
    fn 裁剪到当天范围() {
        let segment = ActiveSegment::new(0, 100);
        assert_eq!(segment.clamp(20, 50), Some(ActiveSegment::new(20, 50)));
        assert_eq!(segment.clamp(200, 300), None);
    }
}
