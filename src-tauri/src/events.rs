//! 前后端事件契约。
//!
//! 后端通过 `emit` 推送、前端通过 `listen` 订阅，名字集中在这里，
//! 避免两端各写一份字面量导致拼写漂移。

pub const LIVE_STATUS: &str = "live-status";
pub const SETTINGS_CHANGED: &str = "settings-changed";
pub const ALARMS_CHANGED: &str = "alarms-changed";
pub const RING_START: &str = "ring-start";
pub const RING_END: &str = "ring-end";
