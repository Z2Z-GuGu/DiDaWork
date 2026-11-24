//! 用户设置。字段全部带默认值，配置文件缺字段时也能启动。

use serde::{Deserialize, Serialize};

/// 关闭闹钟的快捷键（左右两侧同名修饰键同时按下）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Shortcut {
    BothShift,
    BothControl,
    BothAlt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Zh,
    En,
    System,
}

/// 界面明暗。`System` 由前端读系统外观决定具体颜色，后端只负责持久化偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// 是否用鼠标活动判定工作。
    pub detect_mouse: bool,
    /// 是否用键盘输入判定工作。
    pub detect_keyboard: bool,
    /// 最长静止时间（分钟）：超过它才算离开。
    pub freeze_minutes: u32,
    /// 铃声最长播放时长（秒）。
    pub ring_max_seconds: u32,
    /// 关闭闹钟的快捷键。
    pub dismiss_shortcut: Shortcut,
    /// 上班闹钟前已处于活跃状态则不提醒。
    pub skip_clock_in_when_active: bool,
    /// 下班闹钟前已处于非活跃状态则不提醒。
    pub skip_clock_out_when_idle: bool,
    /// 下班后仍在工作是否持续提醒。
    pub overtime_reminder: bool,
    /// 持续提醒间隔（分钟）。
    pub overtime_interval_minutes: u32,
    pub language: Language,
    /// 界面明暗。
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            detect_mouse: true,
            detect_keyboard: true,
            freeze_minutes: 5,
            ring_max_seconds: 60,
            dismiss_shortcut: Shortcut::BothShift,
            skip_clock_in_when_active: true,
            skip_clock_out_when_idle: true,
            overtime_reminder: true,
            overtime_interval_minutes: 10,
            language: Language::Zh,
            theme: Theme::System,
        }
    }
}

impl Settings {
    pub fn freeze_ms(&self) -> i64 {
        self.freeze_minutes as i64 * 60_000
    }

    /// 把外部传入的数值收敛到合理区间，避免出现 0 分钟冻结、10 小时铃声之类。
    pub fn sanitize(&mut self) {
        self.freeze_minutes = self.freeze_minutes.clamp(1, 60);
        self.ring_max_seconds = self.ring_max_seconds.clamp(5, 300);
        self.overtime_interval_minutes = self.overtime_interval_minutes.clamp(1, 180);
        if !self.detect_mouse && !self.detect_keyboard {
            // 两种信号都关掉就永远检测不到活跃，兜底保留键盘。
            self.detect_keyboard = true;
        }
    }
}
