//! 铃声播放。
//!
//! 不引入音频解码库（那会带来几十 MB 的依赖和常驻线程），而是直接调用系统播放器：
//! macOS `afplay`、Windows PowerShell + WPF `MediaPlayer`、Linux `ffplay` / `mpv` / …。
//! 每次响铃只在需要时起一个短命子进程，响完即退。
//!
//! 播的是用户目录里的音乐文件（`ringtones/<名字>/*.m4a|mp3`），不再是系统自带音效。

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// 子进程轮询间隔：决定"按下快捷键到停止发声"的延迟上界。
const POLL_INTERVAL: Duration = Duration::from_millis(60);
/// 子进程活不到这么久就退出，说明格式不支持或播放器起不来，别疯狂重启。
const MIN_PLAY_TIME: Duration = Duration::from_millis(500);

pub struct SoundHandle {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SoundHandle {
    /// 循环播放一首铃声，最多 `max_secs` 秒或调用 `stop()` 结束。
    pub fn play_file(path: &Path, max_secs: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let path = path.to_path_buf();
        let thread = thread::spawn(move || play_loop(&path, max_secs, &flag));
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

fn play_loop(path: &Path, max_secs: u64, stop: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(max_secs);
    while !stop.load(Ordering::Relaxed) && Instant::now() < deadline {
        let started = Instant::now();
        let Some(mut child) = spawn_player(path) else {
            break;
        };
        // 一首播完后接着播下一遍，直到到时或被叫停。
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
        if started.elapsed() < MIN_PLAY_TIME {
            // 播放器一启动就退出了（文件坏了 / 解码器缺失），再试也是白试。
            break;
        }
    }
}

fn spawn_player(path: &Path) -> Option<Child> {
    for mut command in player_commands(path) {
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
fn player_commands(path: &Path) -> Vec<Command> {
    // afplay 是系统自带的，m4a / mp3 都认，播完自己退出。
    let mut play = Command::new("afplay");
    play.arg(path);
    vec![play]
}

#[cfg(target_os = "windows")]
fn player_commands(path: &Path) -> Vec<Command> {
    // SoundPlayer 只认 wav，这里用 WPF 的 MediaPlayer（系统自带的 Media Foundation 解码）。
    // 等它读出时长再播，播完自动退出，播放循环就能接上下一遍。
    let script = format!(
        "Add-Type -AssemblyName PresentationCore; \
         $p = New-Object System.Windows.Media.MediaPlayer; \
         $p.Open([uri]'{}'); \
         $deadline = (Get-Date).AddSeconds(5); \
         while (-not $p.NaturalDuration.HasTimeSpan -and (Get-Date) -lt $deadline) {{ Start-Sleep -Milliseconds 100 }}; \
         $p.Play(); \
         $ms = if ($p.NaturalDuration.HasTimeSpan) {{ [int]$p.NaturalDuration.TimeSpan.TotalMilliseconds }} else {{ 0 }}; \
         Start-Sleep -Milliseconds ($ms + 300); \
         $p.Stop()",
        path.display()
    );
    let mut play = Command::new("powershell");
    play.args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script]);
    vec![play]
}

#[cfg(target_os = "linux")]
fn player_commands(path: &Path) -> Vec<Command> {
    // 发行版差异太大，按"最可能装了"的顺序试；起不来就换下一个。
    let mut ffplay = Command::new("ffplay");
    ffplay.args(["-nodisp", "-autoexit", "-loglevel", "quiet"]).arg(path);

    let mut mpv = Command::new("mpv");
    mpv.args(["--no-video", "--really-quiet"]).arg(path);

    let mut gst = Command::new("gst-play-1.0");
    gst.arg("--quiet").arg(path);

    let mut vlc = Command::new("cvlc");
    vlc.args(["--play-and-exit", "--quiet"]).arg(path);

    vec![ffplay, mpv, gst, vlc]
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn player_commands(path: &Path) -> Vec<Command> {
    let _ = path;
    Vec::new()
}

/// 设置页试听用的单例播放器：同一时刻只允许一个试听。
static PREVIEW: OnceLock<Mutex<Option<SoundHandle>>> = OnceLock::new();

pub fn preview(path: &Path) {
    let slot = PREVIEW.get_or_init(|| Mutex::new(None));
    let mut guard = slot.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(mut previous) = guard.take() {
        previous.stop();
    }
    *guard = Some(SoundHandle::play_file(path, 5));
}

/// 关窗 / 退出时顺手把试听停掉。
pub fn stop_preview() {
    if let Some(slot) = PREVIEW.get() {
        let mut guard = slot.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(mut previous) = guard.take() {
            previous.stop();
        }
    }
}
