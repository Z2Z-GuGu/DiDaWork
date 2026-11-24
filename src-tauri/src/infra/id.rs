//! 轻量 ID 生成：时间戳 + 进程内自增，天然有序且不需要额外依赖。

use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next(prefix: &str) -> String {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let stamp = crate::domain::time::now_ms();
    format!("{prefix}-{stamp:X}-{seq:02X}")
}
