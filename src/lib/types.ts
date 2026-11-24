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

/**
 * 一首铃声 = 数据目录里 `ringtones/` 下的一个文件夹。
 * 文件夹里有 m4a/mp3 就算一首铃声，有 txt 就有版权声明。
 */
export interface Ringtone {
  /** 文件夹名，也是闹钟里存的铃声 id */
  id: string;
  /** 列表里显示的名字，就是文件夹名 */
  name: string;
  hasLicense: boolean;
}

/** 导入自定义铃声的入参：音乐名必填，版权声明可选。 */
export interface RingtoneInput {
  name: string;
  audioPath: string;
  licensePath?: string | null;
}

/** `add_ringtone` 的返回值：刷新后的列表 + 刚创建的那一首。 */
export interface AddedRingtone {
  ringtones: Ringtone[];
  created: Ringtone;
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
