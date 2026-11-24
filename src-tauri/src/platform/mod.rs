//! 平台兼容层。
//!
//! 上层（service / commands）只依赖这里的 trait，不关心操作系统差异：
//!
//! ```text
//!   ┌─────────────────────── service ───────────────────────┐
//!   │  tracker: 每分钟调用一次 InputMonitor::sample()        │
//!   │  ringer:  响铃时轮询 ShortcutWatcher::pressed()        │
//!   └───────────────┬───────────────────────┬───────────────┘
//!                   │                       │
//!    大平台实现 ─────┴───────┐     通用能力 ──┴──┐
//!    macos / windows / linux         audio / notify
//! ```
//!
//! 三个平台的能力差异（诚实说明）：
//! * macOS：用 `CGEventSourceCounterForEventType` 分别统计鼠标与键盘事件数，能做到真正的"键鼠分离"；
//! * Windows：`GetLastInputInfo` 只提供最后一次输入时间，鼠标/键盘不区分；
//! * Linux：XScreenSaver 空闲时间同样不区分。
//! 因此这两个平台的 `mouse` / `keyboard` 字段会同时为真，关闭任一项开关仍按总输入判定。

pub mod audio;
pub mod notify;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

use crate::domain::settings::Shortcut;

/// 一次采样得到的活动信号。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActivitySample {
    pub mouse: bool,
    pub keyboard: bool,
}

impl ActivitySample {    /// 按设置里的开关过滤，得到最终"是否在工作"的判定。
    pub fn filtered(self, detect_mouse: bool, detect_keyboard: bool) -> bool {
        (detect_mouse && self.mouse) || (detect_keyboard && self.keyboard)
    }
}

/// 键鼠活动采集器。实现内部可以保存上一次的计数器用于求差值。
pub trait InputMonitor: Send {
    /// 自上次采样以来是否出现过键鼠活动。
    fn sample(&mut self) -> ActivitySample;
}

/// 修饰键组合监听（用于"同时按下左右 Shift 关闭闹钟"）。
pub trait ShortcutWatcher: Send {
    fn pressed(&mut self, shortcut: Shortcut) -> bool;
}

pub fn input_monitor() -> Box<dyn InputMonitor> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacInputMonitor::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsInputMonitor)
    }
    #[cfg(target_os = "linux")]
    {
        linux::LinuxInputMonitor::new()
            .map(|monitor| Box::new(monitor) as Box<dyn InputMonitor>)
            .unwrap_or_else(|| Box::new(UnsupportedInputMonitor))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(UnsupportedInputMonitor)
    }
}

pub fn shortcut_watcher() -> Box<dyn ShortcutWatcher> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacShortcutWatcher)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsShortcutWatcher)
    }
    #[cfg(target_os = "linux")]
    {
        linux::LinuxShortcutWatcher::new()
            .map(|watcher| Box::new(watcher) as Box<dyn ShortcutWatcher>)
            .unwrap_or_else(|| Box::new(UnsupportedShortcutWatcher))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(UnsupportedShortcutWatcher)
    }
}

/// 不支持的平台或 X11 不可用时的兜底实现：永远"检测不到活动"，功能降级但不崩溃。
#[allow(dead_code)]
pub struct UnsupportedInputMonitor;

impl InputMonitor for UnsupportedInputMonitor {
    fn sample(&mut self) -> ActivitySample {
        ActivitySample::default()
    }
}

#[allow(dead_code)]
pub struct UnsupportedShortcutWatcher;

impl ShortcutWatcher for UnsupportedShortcutWatcher {
    fn pressed(&mut self, _shortcut: Shortcut) -> bool {
        false
    }
}

/// 用系统默认浏览器打开链接。
pub fn open_external(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");

    command.arg(url).spawn().map(|_| ())
}

/// 平台名，展示在设置页。
pub fn name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macOS"
    }
    #[cfg(target_os = "windows")]
    {
        "Windows"
    }
    #[cfg(target_os = "linux")]
    {
        "Linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "Unknown"
    }
}
