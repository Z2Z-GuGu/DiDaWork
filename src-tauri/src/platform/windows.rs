//! Windows 实现。
//!
//! `GetLastInputInfo` 是最省资源的做法：系统已经替我们记录了"最后一次输入"的 tick，
//! 我们只需要和当前 tick 相减得到静止时长。缺点是分不清鼠标还是键盘。

use super::{ActivitySample, InputMonitor, ShortcutWatcher};
use crate::domain::settings::Shortcut;

const VK_LSHIFT: i32 = 0xA0;
const VK_RSHIFT: i32 = 0xA1;
const VK_LCONTROL: i32 = 0xA2;
const VK_RCONTROL: i32 = 0xA3;
const VK_LMENU: i32 = 0xA4;
const VK_RMENU: i32 = 0xA5;

/// 采样周期（毫秒）。静止时长小于它就说明这个周期内有输入。
const SAMPLE_INTERVAL_MS: u32 = 60_000;

#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    time: u32,
}

impl LastInputInfo {
    fn new() -> Self {
        Self {
            cb_size: std::mem::size_of::<LastInputInfo>() as u32,
            time: 0,
        }
    }
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetLastInputInfo(info: *mut LastInputInfo) -> i32;
    fn GetAsyncKeyState(key: i32) -> i16;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetTickCount() -> u32;
}

fn idle_ms() -> u32 {
    let mut info = LastInputInfo::new();
    let ok = unsafe { GetLastInputInfo(&mut info) };
    if ok == 0 {
        return SAMPLE_INTERVAL_MS;
    }
    unsafe { GetTickCount().wrapping_sub(info.time) }
}

fn key_down(key: i32) -> bool {
    unsafe { GetAsyncKeyState(key) as u16 & 0x8000 != 0 }
}

pub struct WindowsInputMonitor;

impl InputMonitor for WindowsInputMonitor {
    fn sample(&mut self) -> ActivitySample {
        // 只有"最近一分钟内有输入"这一个信号，鼠标与键盘同时置位。
        let active = idle_ms() < SAMPLE_INTERVAL_MS;
        ActivitySample {
            mouse: active,
            keyboard: active,
        }
    }
}

pub struct WindowsShortcutWatcher;

impl ShortcutWatcher for WindowsShortcutWatcher {
    fn pressed(&mut self, shortcut: Shortcut) -> bool {
        let (left, right) = match shortcut {
            Shortcut::BothShift => (VK_LSHIFT, VK_RSHIFT),
            Shortcut::BothControl => (VK_LCONTROL, VK_RCONTROL),
            Shortcut::BothAlt => (VK_LMENU, VK_RMENU),
        };
        key_down(left) && key_down(right)
    }
}
