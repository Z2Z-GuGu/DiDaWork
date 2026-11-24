/**
 * 浏览器预览用的假数据。
 *
 * 目的只有一个：不启动 Tauri 也能在浏览器里调界面（`npm run dev` 后直接打开
 * http://localhost:1420）。真实的读取/落盘逻辑全在 Rust 侧，这里只是把命令的
 * 返回值替换成一份稳定的演示数据，**打包产物不包含这段代码**。
 */

import { api } from "@/lib/api";
import type {
  ActiveSegment,
  Alarm,
  AppInfo,
  DaySummary,
  Heatmap,
  LiveStatus,
  Ringtone,
  Settings,
  TimelineDay,
  Trend,
} from "@/lib/types";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

const pad = (value: number) => `${value}`.padStart(2, "0");
const dateKey = (date: Date) =>
  `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
const dayStart = (date: Date) => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
const shiftDays = (date: Date, delta: number) => {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + delta);
  return copy;
};

const TODAY = new Date();
const TODAY_KEY = dateKey(TODAY);
const NOW = Date.now();

/** 生成某天的工作时段：工作日三块，周六偶尔，周日休息。 */
function segmentsFor(date: Date): ActiveSegment[] {
  const key = dateKey(date);
  const weekday = date.getDay();
  const dayIndex = Math.floor(dayStart(date) / DAY);

  if (weekday === 0) return [];
  if (weekday === 6 && dayIndex % 3 !== 0) return [];

  const base = dayStart(date);
  const startHour = 8 + (dayIndex % 5) * 0.25;
  const blocks: Array<[number, number]> = [
    [startHour, startHour + 1.9],
    [startHour + 2.4, startHour + 5.2],
    [startHour + 5.9, startHour + 7.5],
  ];

  return blocks
    .map(([from, to]) => ({
      startMs: base + Math.round(from * HOUR),
      endMs: base + Math.round(to * HOUR),
    }))
    .filter((segment) => key !== TODAY_KEY || segment.startMs < NOW)
    .map((segment) => ({
      startMs: segment.startMs,
      endMs: key === TODAY_KEY ? Math.min(segment.endMs, NOW - 4 * 60_000) : segment.endMs,
    }))
    .filter((segment) => segment.endMs > segment.startMs);
}

const totalOf = (segments: ActiveSegment[]) =>
  segments.reduce((sum, segment) => sum + (segment.endMs - segment.startMs), 0);

function summaryOf(date: Date): DaySummary {
  const segments = segmentsFor(date);
  const key = dateKey(date);
  const isToday = key === TODAY_KEY;
  return {
    date: key,
    activeMs: totalOf(segments),
    segments,
    firstStartMs: segments[0]?.startMs ?? null,
    lastEndMs: segments.length ? segments[segments.length - 1].endMs : null,
    longestMs: segments.reduce((max, segment) => Math.max(max, segment.endMs - segment.startMs), 0),
    live: isToday && segments.length > 0,
    isToday,
  };
}

let settings: Settings = {
  detectMouse: true,
  detectKeyboard: true,
  freezeMinutes: 5,
  ringMaxSeconds: 60,
  dismissShortcut: "bothShift",
  skipClockInWhenActive: true,
  skipClockOutWhenIdle: true,
  overtimeReminder: true,
  overtimeIntervalMinutes: 10,
  language: "zh",
  theme: "system",
};

let alarms: Alarm[] = [
  {
    id: "morning",
    label: "上午办公",
    clockIn: "09:00",
    clockOut: "12:00",
    ringtoneIn: "glass",
    ringtoneOut: "ping",
    weekdays: [true, true, true, true, true, false, false],
    enabled: true,
  },
  {
    id: "afternoon",
    label: "下午办公",
    clockIn: "13:30",
    clockOut: "18:30",
    ringtoneIn: "pop",
    ringtoneOut: "hero",
    weekdays: [true, true, true, true, true, false, false],
    enabled: true,
  },
];

const RINGTONES: Ringtone[] = [
  "glass",
  "ping",
  "pop",
  "tink",
  "hero",
  "submarine",
  "morse",
  "purr",
  "funk",
  "bottle",
].map((id) => ({ id, nameZh: id, nameEn: id }));

const INFO: AppInfo = {
  version: "0.1.0",
  platform: "macOS",
  dataDir: "~/Library/Application Support/com.bugu.didawork",
  repository: "https://github.com/bugu/DiDaWork",
};

function lastSegmentStart(): number | null {
  const segments = summaryOf(TODAY).segments;
  return segments.length ? segments[segments.length - 1].startMs : null;
}

const LIVE: LiveStatus = {
  isActive: true,
  segmentStartMs: lastSegmentStart(),
  todayActiveMs: summaryOf(TODAY).activeMs,
  lastSampleMs: NOW,
  lastInputMs: NOW - 25_000,
};

function trendOf(days: number, end?: string): Trend {
  const last = end ? new Date(`${end}T00:00:00`) : TODAY;
  const points = Array.from({ length: days }, (_, index) => {
    const date = shiftDays(last, index - (days - 1));
    return { date: dateKey(date), activeMs: summaryOf(date).activeMs };
  });
  const totalMs = points.reduce((sum, point) => sum + point.activeMs, 0);
  const working = points.filter((point) => point.activeMs > 0);
  return {
    points,
    totalMs,
    averageMs: working.length ? Math.round(totalMs / working.length) : 0,
    maxMs: points.reduce((max, point) => Math.max(max, point.activeMs), 0),
  };
}

function timelineOf(days: number, end?: string): TimelineDay[] {
  const last = end ? new Date(`${end}T00:00:00`) : TODAY;
  return Array.from({ length: days }, (_, index) => {
    const date = shiftDays(last, index - (days - 1));
    const summary = summaryOf(date);
    return { date: summary.date, activeMs: summary.activeMs, segments: summary.segments };
  });
}

function heatmapOf(weeks: number): Heatmap {
  const thresholds = [1.5 * HOUR, 3 * HOUR, 4.5 * HOUR, 6 * HOUR, 7.5 * HOUR];
  const cells = Array.from({ length: weeks * 7 }, (_, index) => {
    const date = shiftDays(TODAY, index - (weeks * 7 - 1));
    const activeMs = summaryOf(date).activeMs;
    const level = activeMs === 0 ? 0 : thresholds.filter((limit) => activeMs >= limit).length;
    return { date: dateKey(date), activeMs, level: Math.min(level, 4) };
  });
  return {
    cells,
    maxMs: Math.max(thresholds[thresholds.length - 1] ?? 0, ...cells.map((cell) => cell.activeMs)),
    thresholds,
    leadingBlanks: 0,
  };
}

/** 只有浏览器预览走到这里；在 Tauri 里保持真实后端。 */
export function install(): void {
  if (typeof window === "undefined" || "__TAURI_INTERNALS__" in window) return;

  // 预览时可以用 `?theme=dark` / `?theme=light` 看另一套配色
  const preview = new URLSearchParams(window.location.search).get("theme");
  if (preview === "light" || preview === "dark" || preview === "system") {
    settings = { ...settings, theme: preview };
  }
  Object.assign(api, {
    getSettings: async () => settings,
    saveSettings: async (next: Settings) => {
      settings = next;
      return settings;
    },
    listAlarms: async () => alarms,
    saveAlarm: async (alarm: Alarm) => {
      const existing = alarms.findIndex((item) => item.id === alarm.id);
      alarms = existing >= 0
        ? alarms.map((item) => (item.id === alarm.id ? alarm : item))
        : [...alarms, alarm];
      return alarms;
    },
    deleteAlarm: async (id: string) => {
      alarms = alarms.filter((item) => item.id !== id);
      return alarms;
    },
    setAlarmEnabled: async (id: string, enabled: boolean) => {
      alarms = alarms.map((item) => (item.id === id ? { ...item, enabled } : item));
      return alarms;
    },
    getLiveStatus: async () => LIVE,
    getDaySummary: async (date?: string) => summaryOf(date ? new Date(`${date}T00:00:00`) : TODAY),
    getTrend: async (days: number, end?: string) => trendOf(days, end),
    getTimeline: async (days: number, end?: string) => timelineOf(days, end),
    getHeatmap: async (weeks: number) => heatmapOf(weeks),
    getAppInfo: async () => INFO,
    listRingtones: async () => RINGTONES,
    previewRingtone: async () => undefined,
    dismissRing: async () => undefined,
    openMainWindow: async () => undefined,
    openUrl: async () => undefined,
    openDataDir: async () => undefined,
    quitApp: async () => undefined,
  });
}
