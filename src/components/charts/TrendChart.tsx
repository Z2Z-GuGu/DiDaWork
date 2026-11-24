import { useI18n } from "@/i18n";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { formatDayLabel, formatDayShort, formatDuration, todayKey } from "@/lib/format";
import type { TrendPoint } from "@/lib/types";

const CHART_HEIGHT = 112;

/**
 * 近半月的活跃时长柱状图。
 * 用 DOM 而非 Canvas：数据量只有十几根柱子，DOM 更容易做悬停与无障碍。
 */
export function TrendChart({
  points,
  averageMs,
  maxMs,
  selected,
  onSelect,
}: {
  points: TrendPoint[];
  averageMs: number;
  maxMs: number;
  selected: string;
  onSelect: (date: string) => void;
}) {
  const { t, lang } = useI18n();
  const peak = Math.max(maxMs, 1);
  const today = todayKey();
  // 平均线位置：以最高柱为 100%
  const averageLine = Math.min(100, (averageMs / peak) * 100);

  return (
    <div>
      <div className="relative" style={{ height: CHART_HEIGHT }}>
        {averageMs > 0 ? (
          <div
            className="pointer-events-none absolute inset-x-0 border-t border-dashed border-hairline-strong"
            style={{ bottom: `${averageLine}%` }}
          >
            <span className="tabular absolute -top-[15px] right-0 rounded bg-surface px-1 text-[10px] text-ink-faint">
              {t("stats.avg")} {formatDuration(averageMs, lang)}
            </span>
          </div>
        ) : null}

        <div className="flex h-full items-end gap-[3px]">
          {points.map((point) => {
            const ratio = point.activeMs / peak;
            const isSelected = point.date === selected;
            const height = point.activeMs > 0 ? Math.max(ratio * 100, 3) : 2;
            return (
              <Tooltip
                key={point.date}
                content={
                  <div className="space-y-0.5">
                    <div className="font-medium">
                      {formatDayLabel(point.date, lang)}
                      {point.date === today ? ` · ${t("common.today")}` : ""}
                    </div>
                    <div className="tabular text-ink-soft">
                      {formatDuration(point.activeMs, lang)}
                    </div>
                  </div>
                }
              >
                <button
                  type="button"
                  onClick={() => onSelect(point.date)}
                  className="group relative flex h-full flex-1 items-end justify-center outline-none"
                  aria-label={formatDayLabel(point.date, lang)}
                >
                  <span
                    className={cn(
                      "w-full max-w-[26px] rounded-[3px] transition-colors",
                      point.activeMs === 0
                        ? "bg-heat-0"
                        : isSelected
                          ? "bg-accent"
                          : "bg-accent/30 group-hover:bg-accent/55",
                      "group-focus-visible:ring-2 group-focus-visible:ring-accent/40",
                    )}
                    style={{ height: `${height}%` }}
                  />
                </button>
              </Tooltip>
            );
          })}
        </div>
      </div>

      <div className="mt-[6px] flex gap-[3px]">
        {points.map((point, index) => (
          <div
            key={point.date}
            className={cn(
              "tabular flex-1 text-center text-[9px]",
              point.date === selected ? "font-medium text-ink" : "text-ink-faint",
            )}
          >
            {index % 2 === points.length % 2 ? formatDayShort(point.date) : ""}
          </div>
        ))}
      </div>
    </div>
  );
}
