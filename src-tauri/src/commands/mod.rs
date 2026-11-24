//! 命令层：前端与后端的唯一接口。
//!
//! 约定：命令只做"参数整理 + 调用能力 + 广播事件"，不写业务规则。

pub mod alarms;
pub mod app;
pub mod settings;
pub mod stats;
