# Changelog

All notable changes to DiDaWork are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-25

First release. A lightweight desktop work timer that lives in the menu bar / system tray:
it samples keyboard and mouse activity once a minute to reconstruct your working hours,
and rings paired clock-in / clock-out alarms with system notifications and music.

### Activity tracking

- Background sampling once per minute, turning a single "any input in the last minute"
  signal back into continuous activity segments.
- Native input detection per platform: `CGEventSource` counters on macOS (mouse and keyboard
  counted separately), `GetLastInputInfo` on Windows, XScreenSaver idle time on Linux.
- Idle is decided from the moment of the **last detected input** rather than the moment of
  judgement, so recorded durations are not inflated by the idle threshold.
- Configurable freeze window: a gap is only treated as "away" after it exceeds the maximum
  idle time.
- Segments are sorted, gaps under 60 seconds are merged, and data is archived per month to
  `activity/YYYY-MM.json` using atomic writes (temp file + rename). Checkpointed every
  5 minutes and flushed on exit, so an abrupt kill loses at most 5 minutes.
- On platforms without X11 (e.g. Wayland) detection degrades to "not collected" instead of
  crashing.

### Statistics

- Today's total active time at a glance.
- Two-week trend chart with daily average.
- Last-seven-days timeline showing when each day's sessions happened.
- GitHub-style yearly heatmap; clicking any cell or row opens that day's detail.

### Alarms

- Multiple paired clock-in / clock-out alarms, each with a label, weekday repeat and an
  enable / pause switch.
- Backend validation rejects overlapping time ranges.
- Skip rules: stay quiet for clock-in when you are already working, and for clock-out when
  you have already left.
- Overtime reminder keeps nudging you at a configurable interval while you are still working
  after clock-out.

### Ringing

- Rings with a **system notification** plus looping music — no self-drawn window.
- Dismiss from any of: the notification's "Dismiss alarm" action, the tray menu item, the
  in-app banner, or by pressing both Shift keys at once (shortcut configurable to
  Shift / Control / Alt).
- Auto-stops after a configurable maximum ring duration; a session timestamp guard prevents
  an old notification's action from stopping a newer ring.
- macOS uses `UNUserNotificationCenter` with a `UNNotificationCategory` action, forces the
  banner even when the app is frontmost via `willPresent`, and withdraws delivered
  notifications when the ring ends.
- Linux uses freedesktop notification actions through `notify-rust`.
- Windows uses WinRT toasts through `tauri-plugin-notification`; action buttons are not
  exposed by the plugin, so dismissal goes through the tray menu or the shortcut.

### Ringtone library

- Ringtones are folders: one folder is one ringtone, the first `.m4a` or `.mp3` by filename
  is the audio, and an optional `.txt` is treated as the license notice.
- Three bundled CC0 tracks (Emotional Piano, Love Story, Medical) shipped as Tauri resources
  and copied into the app data directory on first launch; a `.seeded` marker keeps deleted
  tracks from coming back.
- "Add custom ringtone" dialog: name (required), audio file (required, `.m4a` / `.mp3`),
  license file (optional, `.txt`). Files are copied into the same folder structure, with name
  sanitising, duplicate detection and path-traversal guards.
- Picker lists names left-aligned; a right-aligned document icon appears when a license
  exists and opens the text file in the default application.
- The library is rescanned every time the picker opens, so folders dropped in Finder /
  Explorer show up without restarting.
- Playback uses short-lived child processes — `afplay` on macOS, PowerShell + WPF
  `MediaPlayer` on Windows, `ffplay` / `mpv` / `gst-play-1.0` / `cvlc` on Linux — looping
  until stopped or until the ring duration elapses.
- Legacy configurations holding system sound ids (`glass`, `ping`, …) are migrated to the
  first entry of the library on startup.

### Settings

- Mouse / keyboard detection toggles and maximum idle time.
- Maximum ring duration and dismiss shortcut.
- Skip rules and overtime reminder interval.
- Appearance: light, dark or follow system.
- Language: Chinese, English or follow system.
- Displays the current data directory and opens it in the file manager.

### Window and system integration

- Resides in the menu bar / system tray: shows today's total, offers "Dismiss alarm"
  (greyed out when idle), reopens the window and quits.
- Closing the window hides it to the tray; data is already on disk.
- Frameless window: macOS keeps the traffic lights over the content with a drag region,
  Windows and Linux get custom window controls.
- Centered "DiDaWork" title in the top bar.
- Custom app icon, plus a monochrome template tray icon on macOS that follows the menu bar's
  light / dark appearance; Windows and Linux keep a white-backed version for contrast.
- All data stays local: `settings.json`, `alarms.json`, `activity/*.json` and `ringtones/`
  under the platform's application data directory.

### Under the hood

- Tauri 2 + Rust backend with strict layering — `commands` / `service` / `domain` / `infra` /
  `platform` — so activity state machines, alarm validation and statistics aggregation are
  pure and unit-tested.
- React 19 + TypeScript + Tailwind CSS v4 with shadcn-style components.
- Chinese / English UI copy and light / dark themes driven by CSS variables.
- A development mock API lets the frontend run in a plain browser without the Rust side.

[0.1.0]: https://github.com/Z2Z-GuGu/DiDaWork/releases/tag/v0.1.0
