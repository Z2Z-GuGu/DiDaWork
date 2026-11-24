//! 进程内共享状态。
//!
//! 所有可变数据都收在这里，用细粒度锁区分读写频率：
//! * `settings` / `alarms` —— 读写都很少，用 `RwLock`；
//! * `tracker` —— 只有采样线程与统计命令会碰，用 `Mutex`；
//! * `ring` —— 响铃会话，同一时刻最多一个。

use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::domain::activity::{coalesce, ActiveSegment, ActivityTracker};
use crate::domain::alarm::Alarm;
use crate::domain::settings::Settings;
use crate::domain::time;
use crate::infra::ringtones::{Library, DIR_NAME as RINGTONES_DIR};
use crate::infra::store::Store;
use crate::service::ringer::RingSession;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    /// 当前是否处于活跃状态（含冻结窗口）。
    pub is_active: bool,
    /// 进行中时段的起点。
    pub segment_start_ms: Option<i64>,
    /// 今天累计活跃毫秒（含进行中时段）。
    pub today_active_ms: i64,
    /// 最后一次采样时刻。
    pub last_sample_ms: i64,
    /// 最后一次检测到键鼠输入的时刻，用于展示"已连续工作多久"。
    pub last_input_ms: Option<i64>,
}

pub struct AppState {
    pub store: Store,
    /// 铃声库：`<数据目录>/ringtones`，一个文件夹一首铃声。
    ringtones: Library,
    settings: RwLock<Settings>,
    alarms: RwLock<Vec<Alarm>>,
    tracker: Mutex<ActivityTracker>,
    live: RwLock<LiveStatus>,
    ring: Mutex<Option<RingSession>>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        let settings = store.load_settings();
        let mut alarms = store.load_alarms();

        let ringtones = Library::new(store.root().join(RINGTONES_DIR));
        if let Err(err) = ringtones.ensure() {
            eprintln!("[state] 创建铃声目录失败: {err}");
        }
        let migrated = migrate_ringtones(&mut alarms, ringtones.list());

        // 首次启动（或文件损坏被重置）时把默认值落盘，用户能直接看到配置文件。
        if let Err(err) = store.save_settings(&settings) {
            eprintln!("[state] 写入设置失败: {err}");
        }
        if migrated {
            eprintln!("[state] 已把失效的铃声换成当前铃声库里的第一首");
        }
        if let Err(err) = store.save_alarms(&alarms) {
            eprintln!("[state] 写入闹钟失败: {err}");
        }

        Self {
            store,
            ringtones,
            settings: RwLock::new(settings),
            alarms: RwLock::new(alarms),
            tracker: Mutex::new(ActivityTracker::new()),
            live: RwLock::new(LiveStatus::default()),
            ring: Mutex::new(None),
        }
    }

    // ---------- 铃声 ----------

    pub fn ringtones(&self) -> &Library {
        &self.ringtones
    }

    // ---------- 设置 ----------

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(|err| err.into_inner()).clone()
    }

    pub fn replace_settings(&self, mut next: Settings) -> Settings {
        next.sanitize();
        {
            let mut guard = self.settings.write().unwrap_or_else(|err| err.into_inner());
            *guard = next.clone();
        }
        let _ = self.store.save_settings(&next);
        next
    }

    // ---------- 闹钟 ----------

    pub fn alarms(&self) -> Vec<Alarm> {
        self.alarms.read().unwrap_or_else(|err| err.into_inner()).clone()
    }

    pub fn replace_alarms(&self, next: Vec<Alarm>) -> Vec<Alarm> {
        {
            let mut guard = self.alarms.write().unwrap_or_else(|err| err.into_inner());
            *guard = next.clone();
        }
        let _ = self.store.save_alarms(&next);
        next
    }

    // ---------- 活跃状态 ----------

    pub fn with_tracker<T>(&self, action: impl FnOnce(&mut ActivityTracker) -> T) -> T {
        let mut guard = self.tracker.lock().unwrap_or_else(|err| err.into_inner());
        action(&mut guard)
    }

    /// 今天的活跃时段：已落盘的 + 进行中的。
    pub fn today_segments(&self) -> (Vec<ActiveSegment>, bool) {
        let today = time::today();
        let mut segments = self.store.segments_on(today);
        let live_segment = self.with_tracker(|tracker| tracker.current_segment());
        let live = live_segment.is_some();
        if let Some(segment) = live_segment {
            let (day_start, day_end) = crate::domain::stats::day_bounds(today);
            if let Some(clipped) = segment.clamp(day_start, day_end) {
                segments.push(clipped);
            }
        }
        coalesce(&mut segments);
        (segments, live)
    }

    pub fn live_status(&self) -> LiveStatus {
        self.live.read().unwrap_or_else(|err| err.into_inner()).clone()
    }

    pub fn refresh_live_status(&self, now_ms: i64) -> LiveStatus {
        let (segments, live) = self.today_segments();
        let status = LiveStatus {
            is_active: live,
            segment_start_ms: self.with_tracker(|tracker| tracker.current_start_ms()),
            today_active_ms: segments.iter().map(|segment| segment.duration_ms()).sum(),
            last_sample_ms: now_ms,
            last_input_ms: segments.last().map(|segment| segment.end_ms),
        };
        {
            let mut guard = self.live.write().unwrap_or_else(|err| err.into_inner());
            *guard = status.clone();
        }
        status
    }

    /// 落盘一段已结束的活跃时间。
    pub fn commit_segment(&self, segment: ActiveSegment) {
        if segment.is_empty() {
            return;
        }
        let date = time::date_of(segment.start_ms);
        let _ = self.store.append_segment(date, segment);
    }

    /// 退出前把进行中的时段写下去，避免丢失最后一段工作记录。
    pub fn flush(&self) {
        let closed = self.with_tracker(|tracker| tracker.force_close());
        if let Some(segment) = closed {
            self.commit_segment(segment);
        }
    }

    // ---------- 响铃会话 ----------

    pub fn ring_slot(&self) -> std::sync::MutexGuard<'_, Option<RingSession>> {
        self.ring.lock().unwrap_or_else(|err| err.into_inner())
    }
}

pub type SharedState = Arc<AppState>;

/// 升级迁移：老版本存的是系统音效名（`glass` / `ping` …），现在铃声库里已经没有了。
/// 把这类失效的 id 指到当前库里的第一首，界面上就不会出现"铃声已失效"。
///
/// 铃声库为空时什么都不做——用户把文件夹清空是主动行为，不去改他的配置。
fn migrate_ringtones(alarms: &mut [Alarm], available: Vec<crate::domain::ringtone::Ringtone>) -> bool {
    if available.is_empty() {
        return false;
    }
    let mut changed = false;
    for alarm in alarms.iter_mut() {
        for slot in [&mut alarm.ringtone_in, &mut alarm.ringtone_out] {
            if let Some(next) = crate::domain::ringtone::normalize(slot.as_str(), &available) {
                if next != slot.as_str() {
                    *slot = next.to_string();
                    changed = true;
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::alarm::ClockTime;
    use crate::domain::ringtone::Ringtone;

    fn alarm_with(ring_in: &str, ring_out: &str) -> Alarm {
        Alarm {
            id: "a".into(),
            label: "工作".into(),
            clock_in: ClockTime::parse("09:00").unwrap(),
            clock_out: ClockTime::parse("18:00").unwrap(),
            ringtone_in: ring_in.into(),
            ringtone_out: ring_out.into(),
            weekdays: vec![true; 7],
            enabled: true,
        }
    }

    fn tone(id: &str) -> Ringtone {
        Ringtone {
            id: id.into(),
            name: id.into(),
            has_license: false,
        }
    }

    #[test]
    fn 失效的铃声会换成第一首() {
        let mut alarms = vec![alarm_with("glass", "voice")];
        let changed = migrate_ringtones(&mut alarms, vec![tone("emotional-piano"), tone("medical")]);
        assert!(changed);
        assert_eq!(alarms[0].ringtone_in, "emotional-piano");
        assert_eq!(alarms[0].ringtone_out, "emotional-piano");
    }

    #[test]
    fn 铃声库为空时不动配置() {
        let mut alarms = vec![alarm_with("glass", "ping")];
        assert!(!migrate_ringtones(&mut alarms, Vec::new()));
        assert_eq!(alarms[0].ringtone_in, "glass");
    }

    #[test]
    fn 本来就有效的铃声保持不变() {
        let mut alarms = vec![alarm_with("medical", "medical")];
        assert!(!migrate_ringtones(&mut alarms, vec![tone("medical")]));
        assert_eq!(alarms[0].ringtone_out, "medical");
    }
}
