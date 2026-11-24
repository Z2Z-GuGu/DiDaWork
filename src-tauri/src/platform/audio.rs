//! 铃声播放。
//!
//! 不引入音频解码库（那会带来几十 MB 的依赖和常驻线程），而是直接调用系统播放器：
//! macOS `afplay`、Windows PowerShell `SoundPlayer`、Linux `paplay` / `aplay`。
//! 每次响铃只在需要时起一个短命子进程，响完即退。

use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// 子进程轮询间隔：决定"按下快捷键到停止发声"的延迟上界。
const POLL_INTERVAL: Duration = Duration::from_millis(60);

pub struct SoundHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SoundHandle {
    /// 循环播放铃声，最多 `max_secs` 秒或调用 `stop()` 结束。
    pub fn play(ringtone: &str, max_secs: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let ringtone = ringtone.to_string();
        let thread = thread::spawn(move || play_loop(&ringtone, max_secs, &flag));
        Self {
            stop,
            thread: Some(thread),
        }
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for SoundHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn play_loop(ringtone: &str, max_secs: u64, stop: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(max_secs);
    while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
        let Some(mut child) = spawn_player(ringtone) else {
            break;
        };
        // 单个音符播完后继续播下一个，直到到时或被叫停。
        loop {
            if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => return,
            }
        }
    }
}

fn spawn_player(ringtone: &str) -> Option<Child> {
    for mut command in player_commands(ringtone) {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(child) = command.spawn() {
            return Some(child);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn player_commands(ringtone: &str) -> Vec<Command> {
    let name = capitalize(ringtone);
    let mut primary = Command::new("afplay");
    primary.arg(format!("/System/Library/Sounds/{name}.aiff"));
    let mut fallback = Command::new("afplay");
    fallback.arg("/System/Library/Sounds/Glass.aiff");
    vec![primary, fallback]
}

#[cfg(target_os = "windows")]
fn player_commands(ringtone: &str) -> Vec<Command> {
    let index = playlist_index(ringtone);
    let path = format!(r"C:\Windows\Media\Alarm{:02}.wav", index);
    let mut primary = Command::new("powershell");
    primary.args([
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &format!(
            "(New-Object Media.SoundPlayer '{path}').PlaySync()"
        ),
    ]);
    let mut fallback = Command::new("powershell");
    fallback.args([
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "-Command",
        "[console]::beep(880,600)",
    ]);
    vec![primary, fallback]
}

#[cfg(target_os = "linux")]
fn player_commands(ringtone: &str) -> Vec<Command> {
    let _ = ringtone;
    let mut paplay = Command::new("paplay");
    paplay.arg("/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga");
    let mut aplay = Command::new("aplay");
    aplay.arg("/usr/share/sounds/alsa/Front_Center.wav");
    let mut canberra = Command::new("canberra-gtk-play");
    canberra.args(["-i", "alarm-clock-elapsed"]);
    vec![paplay, aplay, canberra]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn player_commands(ringtone: &str) -> Vec<Command> {
    let _ = ringtone;
    Vec::new()
}

#[cfg(target_os = "macos")]
fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Glass".to_string(),
    }
}

#[cfg(target_os = "windows")]
fn playlist_index(ringtone: &str) -> u8 {
    crate::domain::ringtone::all()
        .iter()
        .position(|item| item.id == ringtone)
        .map(|index| (index as u8) + 1)
        .unwrap_or(1)
}

/// 设置页试听用的单例播放器：同一时刻只允许一个试听。
static PREVIEW: OnceLock<Mutex<Option<SoundHandle>>> = OnceLock::new();

pub fn preview(ringtone: &str) {
    let slot = PREVIEW.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(mut previous) = guard.take() {
        previous.stop();
    }
    *guard = Some(SoundHandle::play(ringtone, 3));
}
