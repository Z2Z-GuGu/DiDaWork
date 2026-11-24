import { useI18n } from "@/i18n";
import { cn } from "@/lib/utils";
import {
  MS_DAY,
  formatDayShort,
  formatDuration,
  formatDurationCompact,
  formatMinuteOfDay,
  startOfDay,
  todayKey,
  weekdayShort,
  weekdayIndex,
} from "@/lib/format";
import type { TimelineDay } from "@/lib/types";

const HOUR_TICKS = [0, 6, 12, 18, 24];

/**
 * 近 7 天的时间段分布：一天一行，横向按 0-24 点铺开。
 * 每段用绝对定位渲染，宽度即持续时间；点击整行可切换查看那一天的详情。
 */
export function TimelineRows({
  days,
  selected,
  onSelect,
}: {
  days: TimelineDay[];
  selected: string;
  onSelect: (date: string) => void;
}) {
  const { t, lang } = useI18n();
  const today = todayKey();

  return (
    <div>
      {/* 刻度：0 / 6 / 12 / 18 / 24 */}
      <div className="mb-1 flex pl-[78px] pr-[62px]">
        <div className="relative h-3 flex-1">
          {HOUR_TICKS.map((hour, index) => (
            <span
              key={hour}
              className="tabular absolute top-0 text-[9px] text-ink-faint"
              style={{
                left: `${(hour / 24) * 100}%`,
                transform:
                  index === 0
                    ? "translateX(0)"
                    : index === HOUR_TICKS.length - 1
                      ? "translateX(-100%)"
                      : "translateX(-50%)",
              }}
            >
              {hour}:00
            </span>
          ))}
        </div>
      </div>

      <div className="flex flex-col gap-[2px]">
        {days.map((day) => {
          const isSelected = day.date === selected;
          const dayStart = startOfDay(day.date);
          return (
            <button
              key={day.date}
              type="button"
              onClick={() => onSelect(day.date)}
              className={cn(
                "flex items-center gap-2.5 rounded-lg py-[3px] pl-1.5 pr-1 text-left transition-colors",
                isSelected ? "bg-accent-soft" : "hover:bg-subtle",
              )}
            >
              <span className="flex w-[62px] shrink-0 items-baseline gap-1">
                <span
                  className={cn(
                    "text-[11px]",
                    isSelected ? "font-medium text-accent" : "text-ink-soft",
                  )}
                >
                  {day.date === today
                    ? t("common.today")
                    : weekdayShort(weekdayIndex(day.date), lang)}
                </span>
                <span className="tabular text-[10px] text-ink-faint">
                  {formatDayShort(day.date)}
                </span>
              </span>

              <span className="relative h-[13px] flex-1 overflow-hidden rounded-[4px] bg-heat-0">
                {day.segments.map((segment) => {
                  const start = Math.max(0, (segment.startMs - dayStart) / MS_DAY);
                  const end = Math.min(1, (segment.endMs - dayStart) / MS_DAY);
                  if (end <= start) return null;
                  return (
                    <span
                      key={`${segment.startMs}-${segment.endMs}`}
                      title={`${formatMinuteOfDay(Math.round(start * 1440))} – ${formatMinuteOfDay(
                        Math.round(end * 1440),
                      )} · ${formatDuration(segment.endMs - segment.startMs, lang)}`}
                      className={cn(
                        "absolute inset-y-0 rounded-[3px]",
                        day.date === today ? "bg-accent" : "bg-accent/55",
                        isSelected && "bg-accent",
                      )}
                      style={{
                        left: `${start * 100}%`,
                        width: `${Math.max((end - start) * 100, 0.4)}%`,
                      }}
                    />
                  );
                })}
              </span>

              <span
                title={day.activeMs > 0 ? formatDuration(day.activeMs, lang) : undefined}
                className={cn(
                  "tabular w-[48px] shrink-0 whitespace-nowrap text-right text-[10px]",
                  day.activeMs > 0 ? "text-ink-soft" : "text-ink-faint",
                )}
              >
                {day.activeMs > 0 ? formatDurationCompact(day.activeMs) : "—"}
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
