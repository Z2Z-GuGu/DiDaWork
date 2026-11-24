//! Linux 实现（X11）。
//!
//! * 活动检测用 XScreenSaver 扩展的空闲时间（`XScreenSaverQueryInfo`），
//!   同样分不清鼠标与键盘，因此 `mouse` / `keyboard` 同时置位。
//! * 快捷键用 `XQueryKeymap` 读取整张键盘状态位图，判断左右修饰键是否同时按下。
//!
//! Wayland 会话下 `DISPLAY` 往往不可用，此时 `XOpenDisplay` 返回空指针，
//! 上层会自动降级为"不采集活动"的空实现，而不是崩溃。

use std::ffi::c_char;

use x11_dl::xlib::{self, Display};
use x11_dl::xss::{self, XScreenSaverInfo};

use super::{ActivitySample, InputMonitor, ShortcutWatcher};
use crate::domain::settings::Shortcut;

/// X11 键码（evdev 布局，left/right 修饰键位置固定）。
const KEY_LEFT_SHIFT: u8 = 50;
const KEY_RIGHT_SHIFT: u8 = 62;
const KEY_LEFT_CONTROL: u8 = 37;
const KEY_RIGHT_CONTROL: u8 = 105;
const KEY_LEFT_ALT: u8 = 64;
const KEY_RIGHT_ALT: u8 = 108;

const SAMPLE_INTERVAL_MS: u64 = 60_000;

struct X11 {
    xlib: xlib::Xlib,
    xss: xss::Xss,
    display: *mut Display,
}

// 裸指针只在 owning 线程内使用（monitor 与 watcher 各自在自己的线程里创建），
// 但 trait 要求 Send，这里显式承诺不做跨线程共享访问。
unsafe impl Send for X11 {}

impl X11 {
    fn open() -> Option<Self> {
        let xlib = xlib::Xlib::open().ok()?;
        let xss = xss::Xss::open().ok()?;
        let display = unsafe { (xlib.XOpenDisplay)(std::ptr::null()) };
        if display.is_null() {
            return None;
        }
        Some(Self { xlib, xss, display })
    }

    fn idle_ms(&self) -> Option<u64> {
        // 让 XScreenSaverInfo 分配在栈上，避免每个采样点都 malloc/free。
        let mut info: XScreenSaverInfo = unsafe { std::mem::zeroed() };
        let root = unsafe { (self.xlib.XRootWindow)(self.display, 0) };
        let status = unsafe { (self.xss.XScreenSaverQueryInfo)(self.display, root, &mut info) };
        (status != 0).then_some(info.idle)
    }

    fn key_down(&self, keycode: u8) -> bool {
        let mut keys = [0 as c_char; 32];
        let ok = unsafe { (self.xlib.XQueryKeymap)(self.display, &mut keys) };
        if ok == 0 {
            return false;
        }
        let byte = keys[(keycode / 8) as usize] as u8;
        byte & (1 << (keycode % 8)) != 0
    }
}

pub struct LinuxInputMonitor {
    x11: X11,
}

impl LinuxInputMonitor {
    pub fn new() -> Option<Self> {
        X11::open().map(|x11| Self { x11 })
    }
}

impl InputMonitor for LinuxInputMonitor {
    fn sample(&mut self) -> ActivitySample {
        let active = self
            .x11
            .idle_ms()
            .map(|idle| idle < SAMPLE_INTERVAL_MS)
            .unwrap_or(false);
        ActivitySample {
            mouse: active,
            keyboard: active,
        }
    }
}

pub struct LinuxShortcutWatcher {
    x11: X11,
}

impl LinuxShortcutWatcher {
    pub fn new() -> Option<Self> {
        X11::open().map(|x11| Self { x11 })
    }
}

impl ShortcutWatcher for LinuxShortcutWatcher {
    fn pressed(&mut self, shortcut: Shortcut) -> bool {
        let (left, right) = match shortcut {
            Shortcut::BothShift => (KEY_LEFT_SHIFT, KEY_RIGHT_SHIFT),
            Shortcut::BothControl => (KEY_LEFT_CONTROL, KEY_RIGHT_CONTROL),
            Shortcut::BothAlt => (KEY_LEFT_ALT, KEY_RIGHT_ALT),
        };
        self.x11.key_down(left) && self.x11.key_down(right)
    }
}
