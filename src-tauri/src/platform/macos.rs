//! macOS 实现。
//!
//! 用 CoreGraphics 的 `CGEventSourceCounterForEventType` 读取"合并会话"的事件计数器，
//! 相比安装事件监听器（CGEventTap）几乎零开销、不需要辅助功能权限，也不常驻回调。
//!
//! 计数器是单调递增的累计值，因此每次采样都与上一次做差得到区间内的事件数。

use super::{ActivitySample, InputMonitor, ShortcutWatcher};
use crate::domain::settings::Shortcut;

type CGEventSourceStateID = i32;

/// `kCGEventSourceStateCombinedSessionState`
const COMBINED_SESSION: CGEventSourceStateID = 0;

// 只关心"有人动过"的事件类型，忽略窗口移动、前台切换之类的系统事件。
const EVENT_LEFT_MOUSE_DOWN: u32 = 1;
const EVENT_RIGHT_MOUSE_DOWN: u32 = 3;
const EVENT_MOUSE_MOVED: u32 = 5;
const EVENT_LEFT_MOUSE_DRAGGED: u32 = 6;
const EVENT_RIGHT_MOUSE_DRAGGED: u32 = 7;
const EVENT_KEY_DOWN: u32 = 10;
const EVENT_SCROLL_WHEEL: u32 = 22;
const EVENT_OTHER_MOUSE_DOWN: u32 = 25;

const MOUSE_EVENTS: [u32; 7] = [
    EVENT_LEFT_MOUSE_DOWN,
    EVENT_RIGHT_MOUSE_DOWN,
    EVENT_MOUSE_MOVED,
    EVENT_LEFT_MOUSE_DRAGGED,
    EVENT_RIGHT_MOUSE_DRAGGED,
    EVENT_OTHER_MOUSE_DOWN,
    EVENT_SCROLL_WHEEL,
];

// 虚拟键码（Carbon HIToolbox 常量），左右两侧是独立的键码。
const KEY_LEFT_SHIFT: u16 = 56;
const KEY_RIGHT_SHIFT: u16 = 60;
const KEY_LEFT_CONTROL: u16 = 59;
const KEY_RIGHT_CONTROL: u16 = 62;
const KEY_LEFT_OPTION: u16 = 58;
const KEY_RIGHT_OPTION: u16 = 61;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceCounterForEventType(source: CGEventSourceStateID, event_type: u32) -> u32;
    fn CGEventSourceKeyState(source: CGEventSourceStateID, key: u16) -> bool;
}

fn counter(event_type: u32) -> u32 {
    unsafe { CGEventSourceCounterForEventType(COMBINED_SESSION, event_type) }
}

fn key_down(keycode: u16) -> bool {
    unsafe { CGEventSourceKeyState(COMBINED_SESSION, keycode) }
}

pub struct MacInputMonitor {
    /// 上一次采样时鼠标事件的累计数（多个事件类型求和）。
    last_mouse: u32,
    last_keyboard: u32,
}

impl MacInputMonitor {
    pub fn new() -> Self {
        Self {
            last_mouse: MOUSE_EVENTS.iter().map(|event| counter(*event)).sum(),
            last_keyboard: counter(EVENT_KEY_DOWN),
        }
    }
}

impl InputMonitor for MacInputMonitor {
    fn sample(&mut self) -> ActivitySample {
        let mouse_total: u32 = MOUSE_EVENTS.iter().map(|event| counter(*event)).sum();
        let keyboard_total = counter(EVENT_KEY_DOWN);

        // 计数器理论上不会回退（重启会话除外），用 wrapping_sub 兜底。
        let sample = ActivitySample {
            mouse: mouse_total != self.last_mouse,
            keyboard: keyboard_total.wrapping_sub(self.last_keyboard) > 0,
        };
        self.last_mouse = mouse_total;
        self.last_keyboard = keyboard_total;
        sample
    }
}

pub struct MacShortcutWatcher;

impl ShortcutWatcher for MacShortcutWatcher {
    fn pressed(&mut self, shortcut: Shortcut) -> bool {
        let (left, right) = match shortcut {
            Shortcut::BothShift => (KEY_LEFT_SHIFT, KEY_RIGHT_SHIFT),
            Shortcut::BothControl => (KEY_LEFT_CONTROL, KEY_RIGHT_CONTROL),
            Shortcut::BothAlt => (KEY_LEFT_OPTION, KEY_RIGHT_OPTION),
        };
        key_down(left) && key_down(right)
    }
}
