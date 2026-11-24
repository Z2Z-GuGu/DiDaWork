//! 持久化层：JSON 文件 + 内存缓存。
//!
//! 为什么不用数据库：单机、单用户、每天几十条记录，JSON 足够；
//! 好处是零依赖、体积小、用户可以自己查看/备份数据。
//!
//! * `settings.json` —— 设置
//! * `alarms.json`   —— 闹钟列表
//! * `activity/YYYY-MM.json` —— 按月归档的活跃时段，形如
//!   `{ "2026-11-24": [{ "startMs": 0, "endMs": 60000 }] }`

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::NaiveDate;

use crate::domain::activity::ActiveSegment;
use crate::domain::alarm::Alarm;
use crate::domain::settings::Settings;
use crate::domain::time::{date_key, end_of_day_ms, month_key, start_of_day_ms};

/// 单月文件内容：日期 → 当天的活跃时段。
type MonthFile = BTreeMap<String, Vec<ActiveSegment>>;
const SETTINGS_FILE: &str = "settings.json";
const ALARMS_FILE: &str = "alarms.json";
const ACTIVITY_DIR: &str = "activity";

pub struct Store {
    root: PathBuf,
    /// 月份文件缓存，避免密集统计时反复读盘。
    cache: Mutex<HashMap<String, MonthFile>>,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn ensure_layout(&self) -> io::Result<()> {
        fs::create_dir_all(self.root.join(ACTIVITY_DIR))?;
        self.clear_stale_temps();
        Ok(())
    }

    /// 上次异常退出可能残留 `.tmp`，启动时顺手清掉。
    fn clear_stale_temps(&self) {
        for dir in [self.root.clone(), self.root.join(ACTIVITY_DIR)] {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "tmp") {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // ---------- 设置 ----------

    pub fn load_settings(&self) -> Settings {
        let mut settings = self
            .read_json::<Settings>(SETTINGS_FILE)
            .unwrap_or_default();
        settings.sanitize();
        settings
    }

    pub fn save_settings(&self, settings: &Settings) -> io::Result<()> {
        self.write_json(SETTINGS_FILE, settings)
    }

    // ---------- 闹钟 ----------

    pub fn load_alarms(&self) -> Vec<Alarm> {
        self.read_json::<Vec<Alarm>>(ALARMS_FILE).unwrap_or_default()
    }

    pub fn save_alarms(&self, alarms: &[Alarm]) -> io::Result<()> {
        self.write_json(ALARMS_FILE, &alarms)
    }

    // ---------- 活跃时段 ----------

    pub fn append_segment(&self, date: NaiveDate, segment: ActiveSegment) -> io::Result<()> {
        if segment.is_empty() {
            return Ok(());
        }
        let (day_start, day_end) = (start_of_day_ms(date), end_of_day_ms(date));
        let Some(segment) = segment.clamp(day_start, day_end) else {
            return Ok(());
        };

        let key = month_key(date);
        let mut cache = self.cache.lock().unwrap_or_else(|err| err.into_inner());
        let file_path = self.month_path(&key);
        let mut month = match cache.get(&key) {
            Some(month) => month.clone(),
            None => self.read_month(&file_path),
        };
        let entry = month.entry(date_key(date)).or_default();
        entry.push(segment);
        crate::domain::activity::coalesce(entry);
        self.write_json(&format!("{ACTIVITY_DIR}/{key}.json"), &month)?;
        cache.insert(key, month);
        Ok(())
    }

    /// 某一天的活跃时段（已裁剪到当天）。
    pub fn segments_on(&self, date: NaiveDate) -> Vec<ActiveSegment> {
        let (day_start, day_end) = (start_of_day_ms(date), end_of_day_ms(date));
        self.month_data(date)
            .get(&date_key(date))
            .map(|segments| {
                segments
                    .iter()
                    .filter_map(|segment| segment.clamp(day_start, day_end))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 区间内逐日的活跃时段，用于趋势 / 时间段 / 热力图。
    pub fn segments_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<(NaiveDate, Vec<ActiveSegment>)> {
        let mut result = Vec::new();
        let mut cursor = from;
        while cursor <= to {
            result.push((cursor, self.segments_on(cursor)));
            match cursor.succ_opt() {
                Some(next) => cursor = next,
                None => break,
            }
        }
        result
    }

    fn month_data(&self, date: NaiveDate) -> MonthFile {
        let key = month_key(date);
        {
            let cache = self.cache.lock().unwrap_or_else(|err| err.into_inner());
            if let Some(cached) = cache.get(&key) {
                return cached.clone();
            }
        }
        let month = self.read_month(&self.month_path(&key));
        self.cache
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(key, month.clone());
        month
    }

    fn month_path(&self, key: &str) -> PathBuf {
        self.root.join(ACTIVITY_DIR).join(format!("{key}.json"))
    }

    fn read_month(&self, path: &Path) -> MonthFile {
        self.read_json_path(path).unwrap_or_default()
    }

    // ---------- JSON 读写 ----------

    fn read_json<T: serde::de::DeserializeOwned>(&self, relative: &str) -> Option<T> {
        self.read_json_path(&self.root.join(relative))
    }

    fn read_json_path<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Option<T> {
        let raw = fs::read(path).ok()?;
        match serde_json::from_slice(&raw) {
            Ok(value) => Some(value),
            Err(err) => {
                // 文件损坏时先备份，避免下次写入把内容彻底覆盖掉用户数据。
                eprintln!("[store] 解析 {} 失败: {err}", path.display());
                let _ = fs::rename(path, path.with_extension("corrupt.json"));
                None
            }
        }
    }

    /// 原子写入：先写临时文件再改名，避免半截文件。
    ///
    /// 少数情况下改名会被系统拒绝（例如目标文件带有 `com.apple.provenance`
    /// 标记、由别的签名程序创建）。这时退化为"删除 → 再改名 → 直接覆盖"，
    /// 宁可少一点原子性，也不要丢数据。
    fn write_json<T: serde::Serialize>(&self, relative: &str, value: &T) -> io::Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut buffer = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        buffer.push(b'\n');

        let temp = path.with_extension("tmp");
        fs::write(&temp, &buffer)?;

        match fs::rename(&temp, &path) {
            Ok(()) => Ok(()),
            Err(first_error) => {
                let _ = fs::remove_file(&path);
                if fs::rename(&temp, &path).is_ok() {
                    return Ok(());
                }
                let _ = fs::remove_file(&temp);
                fs::write(&path, &buffer).map_err(|_| first_error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 相邻碎片会被合并() {
        let mut segments = vec![
            ActiveSegment::new(0, 60_000),
            ActiveSegment::new(120_000, 180_000),
            ActiveSegment::new(500_000, 560_000),
        ];
        crate::domain::activity::coalesce(&mut segments);
        assert_eq!(
            segments,
            vec![
                ActiveSegment::new(0, 180_000),
                ActiveSegment::new(500_000, 560_000),
            ]
        );
    }

    #[test]
    fn 原子写入不留临时文件() {
        let dir = std::env::temp_dir().join("didawork-store-atomic");
        let _ = fs::remove_dir_all(&dir);
        let store = Store::new(&dir);
        store.ensure_layout().unwrap();

        store
            .save_settings(&crate::domain::settings::Settings::default())
            .unwrap();
        assert!(dir.join("settings.json").exists(), "settings.json 应存在");
        assert!(!dir.join("settings.tmp").exists(), "不应残留临时文件");

        // 覆盖写：目标文件已存在时也要成功替换
        let mut settings = crate::domain::settings::Settings::default();
        settings.freeze_minutes = 9;
        store.save_settings(&settings).unwrap();
        assert!(!dir.join("settings.tmp").exists(), "覆盖后不应残留临时文件");
        let reloaded: crate::domain::settings::Settings =
            store.read_json("settings.json").expect("应能读回");
        assert_eq!(reloaded.freeze_minutes, 9, "覆盖写应生效");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn 落盘后可以读回() {
        let dir = std::env::temp_dir().join("didawork-store-test");
        let _ = fs::remove_dir_all(&dir);
        let store = Store::new(&dir);
        store.ensure_layout().unwrap();

        let date = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        let day_start = start_of_day_ms(date);
        store
            .append_segment(date, ActiveSegment::new(day_start, day_start + 3_600_000))
            .unwrap();

        let loaded = store.segments_on(date);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].duration_ms(), 3_600_000);
        let _ = fs::remove_dir_all(&dir);
    }
}
