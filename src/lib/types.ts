/**
 * 与 Rust 后端一一对应的数据结构（serde camelCase）。
 * 改动后端 `domain` / `commands` 里的结构体时，同步改这里。
 */

export type Language = "zh" | "en" | "system";
export type Shortcut = "bothShift" | "bothControl" | "bothAlt";
/** 界面明暗：跟随系统 / 浅色 / 深色 */
export type Theme = "system" | "light" | "dark";

export interface Settings {
  detectMouse: boolean;
  detectKeyboard: boolean;
  freezeMinutes: number;
  ringMaxSeconds: number;
  dismissShortcut: Shortcut;
  skipClockInWhenActive: boolean;
  skipClockOutWhenIdle: boolean;
  overtimeReminder: boolean;
  overtimeIntervalMinutes: number;
  language: Language;
  theme: Theme;
}

export type AlarmKind = "clockIn" | "clockOut" | "overtime";

export interface Alarm {
  id: string;
  label: string;
  /** `HH:MM` */
  clockIn: string;
  /** `HH:MM` */
  clockOut: string;
  ringtoneIn: string;
  ringtoneOut: string;
  /** 周一 → 周日，长度固定 7 */
  weekdays: boolean[];
  enabled: boolean;
}

export interface ActiveSegment {
  startMs: number;
  endMs: number;
}

export interface DaySummary {
  date: string;
  activeMs: number;
  segments: ActiveSegment[];
  firstStartMs: number | null;
  lastEndMs: number | null;
  longestMs: number;
  live: boolean;
  isToday: boolean;
}

export interface TrendPoint {
  date: string;
  activeMs: number;
}

export interface Trend {
  points: TrendPoint[];
  totalMs: number;
  averageMs: number;
  maxMs: number;
}

export interface TimelineDay {
  date: string;
  activeMs: number;
  segments: ActiveSegment[];
}

export interface HeatmapCell {
  date: string;
  activeMs: number;
  level: number;
}

export interface Heatmap {
  cells: HeatmapCell[];
  maxMs: number;
  thresholds: number[];
  leadingBlanks: number;
}

export interface LiveStatus {
  isActive: boolean;
  segmentStartMs: number | null;
  todayActiveMs: number;
  lastSampleMs: number;
  lastInputMs: number | null;
}

export interface Ringtone {
  id: string;
  nameZh: string;
  nameEn: string;
}

export interface AppInfo {
  version: string;
  platform: string;
  dataDir: string;
  repository: string;
}

export interface RingInfo {
  alarmId: string;
  kind: AlarmKind;
  label: string;
  ringtone: string;
  startedMs: number;
  clock: string;
}

/** 后端校验失败的载荷。 */
export interface ValidationError {
  code:
    | "labelRequired"
    | "weekdaysRequired"
    | "weekdaysInvalid"
    | "rangeInvalid"
    | "overlap"
    | string;
  message: string;
  conflicts: string[];
}
