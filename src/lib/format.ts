/**
 * 日期与时长格式化。
 * 全部按"本地时间"处理，和后端保持一致（后端存的是毫秒时间戳 + 本地日期键）。
 */

export type Lang = "zh" | "en";

export const MS_MINUTE = 60_000;
export const MS_HOUR = 3_600_000;
export const MS_DAY = 86_400_000;

/** `YYYY-MM-DD`（本地） */
export function dateKey(date: Date): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function todayKey(): string {
  return dateKey(new Date());
}

/** `YYYY-MM-DD` → 本地当天 0 点 */
export function parseKey(key: string): Date {
  const [year, month, day] = key.split("-").map(Number);
  return new Date(year, (month ?? 1) - 1, day ?? 1);
}

export function addDays(key: string, delta: number): string {
  const date = parseKey(key);
  date.setDate(date.getDate() + delta);
  return dateKey(date);
}

/** 周一 = 0，周日 = 6 */
export function weekdayIndex(key: string): number {
  return (parseKey(key).getDay() + 6) % 7;
}

/** 该天 0 点的时间戳 */
export function startOfDay(key: string): number {
  return parseKey(key).getTime();
}

/** 时长 → "3 小时 20 分" / "3h 20m" */
export function formatDuration(ms: number, lang: Lang = "zh"): string {
  const minutes = Math.max(0, Math.round(ms / MS_MINUTE));
  if (minutes === 0) return lang === "zh" ? "0 分钟" : "0m";
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (lang === "zh") {
    if (hours === 0) return `${rest} 分钟`;
    return rest === 0 ? `${hours} 小时` : `${hours} 小时 ${rest} 分`;
  }
  if (hours === 0) return `${rest}m`;
  return rest === 0 ? `${hours}h` : `${hours}h ${rest}m`;
}

/** 时长 → "3:20"（坐标轴、紧凑卡片用） */
export function formatDurationCompact(ms: number): string {
  const minutes = Math.max(0, Math.round(ms / MS_MINUTE));
  return `${Math.floor(minutes / 60)}:${`${minutes % 60}`.padStart(2, "0")}`;
}

/** 时间戳 → "14:05" */
export function formatClock(ms: number): string {
  const date = new Date(ms);
  return `${`${date.getHours()}`.padStart(2, "0")}:${`${date.getMinutes()}`.padStart(2, "0")}`;
}

/** 一天中的第几分钟 → "14:05" */
export function formatMinuteOfDay(minute: number): string {
  const total = ((minute % 1440) + 1440) % 1440;
  return `${`${Math.floor(total / 60)}`.padStart(2, "0")}:${`${total % 60}`.padStart(2, "0")}`;
}

const ZH_WEEKDAYS = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"];
const EN_WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const EN_MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

/** "11月24日 周一" / "Nov 24, Mon" */
export function formatDayLabel(key: string, lang: Lang = "zh"): string {
  const date = parseKey(key);
  const index = weekdayIndex(key);
  if (lang === "zh") {
    return `${date.getMonth() + 1}月${date.getDate()}日 ${ZH_WEEKDAYS[index]}`;
  }
  return `${EN_MONTHS[date.getMonth()]} ${date.getDate()}, ${EN_WEEKDAYS[index]}`;
}

/** "11/24" / "11/24" */
export function formatDayShort(key: string): string {
  const date = parseKey(key);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

/** 星期几的短名 */
export function weekdayShort(index: number, lang: Lang = "zh"): string {
  return lang === "zh" ? ZH_WEEKDAYS[index] : EN_WEEKDAYS[index];
}

/** 把分钟数格式化成可读说明，用于"冻结 5 分钟"这类文案 */
export function formatMinutes(minutes: number, lang: Lang = "zh"): string {
  return lang === "zh" ? `${minutes} 分钟` : `${minutes} min`;
}

/** `HH:MM` → 分钟数 */
export function clockToMinutes(clock: string): number {
  const [hour, minute] = clock.split(":").map(Number);
  return (hour || 0) * 60 + (minute || 0);
}

/** 相对时间："刚刚" / "3 分钟前" */
export function formatRelative(ms: number, lang: Lang = "zh"): string {
  const diff = Date.now() - ms;
  if (diff < MS_MINUTE) return lang === "zh" ? "刚刚" : "just now";
  const minutes = Math.round(diff / MS_MINUTE);
  return lang === "zh" ? `${minutes} 分钟前` : `${minutes} min ago`;
}
