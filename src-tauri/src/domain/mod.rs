//! 领域层：与平台、存储完全解耦的纯逻辑。
//!
//! 这里只放"业务规则"——活跃状态机、闹钟模型与校验、统计聚合、设置默认值。
//! 任何需要操作系统能力或读写磁盘的事情都不属于这一层，方便单元测试与替换实现。

pub mod activity;
pub mod alarm;
pub mod ringtone;
pub mod settings;
pub mod stats;
pub mod time;
