/**
 * 后端命令的类型化封装。
 * 组件不直接调用 invoke，统一走这里，方便改签名与统一错误处理。
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  AddedRingtone,
  Alarm,
  AppInfo,
  DaySummary,
  Heatmap,
  LiveStatus,
  Ringtone,
  RingtoneInput,
  Settings,
  TimelineDay,
  Trend,
  ValidationError,
} from "./types";

export const api = {
  // ---- 设置 ----
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),

  // ---- 闹钟 ----
  listAlarms: () => invoke<Alarm[]>("list_alarms"),
  saveAlarm: (alarm: Alarm) => invoke<Alarm[]>("save_alarm", { alarm }),
  deleteAlarm: (id: string) => invoke<Alarm[]>("delete_alarm", { id }),
  setAlarmEnabled: (id: string, enabled: boolean) =>
    invoke<Alarm[]>("set_alarm_enabled", { id, enabled }),

  // ---- 统计 ----
  getLiveStatus: () => invoke<LiveStatus>("get_live_status"),
  getDaySummary: (date?: string) => invoke<DaySummary>("get_day_summary", { date }),
  getTrend: (days: number, end?: string) => invoke<Trend>("get_trend", { days, end }),
  getTimeline: (days: number, end?: string) =>
    invoke<TimelineDay[]>("get_timeline", { days, end }),
  getHeatmap: (weeks: number) => invoke<Heatmap>("get_heatmap", { weeks }),

  // ---- 应用 ----
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  listRingtones: () => invoke<Ringtone[]>("list_ringtones"),
  previewRingtone: (id: string) => invoke<void>("preview_ringtone", { id }),
  addRingtone: (input: RingtoneInput) =>
    invoke<AddedRingtone>("add_ringtone", {
      name: input.name,
      audioPath: input.audioPath,
      licensePath: input.licensePath ?? null,
    }),
  openRingtoneLicense: (id: string) => invoke<void>("open_ringtone_license", { id }),
  dismissRing: () => invoke<void>("dismiss_ring"),
  openMainWindow: () => invoke<void>("open_main_window"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  openDataDir: () => invoke<void>("open_data_dir"),
  quitApp: () => invoke<void>("quit_app"),
};

/** 判断是否为后端返回的校验错误。 */
export function isValidationError(error: unknown): error is ValidationError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as ValidationError).code === "string"
  );
}

/** 把任意异常转成可读文本。 */
export function toMessage(error: unknown): string {
  if (isValidationError(error)) return error.message;
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
