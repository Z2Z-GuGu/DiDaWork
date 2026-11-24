import { useI18n } from "@/i18n";
import { cn } from "@/lib/utils";
import { formatDayLabel, formatDuration, todayKey } from "@/lib/format";
import type { Heatmap as HeatmapData, HeatmapCell } from "@/lib/types";

const LEVEL_CLASS = [
  "bg-heat-0",
  "bg-heat-1",
  "bg-heat-2",
  "bg-heat-3",
  "bg-heat-4",
] as const;

const MONTH_NAMES = [
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

/** 单元格 8px + 间距 3px，53 列约 583px，能放进 960 宽的窗口。 */
const CELL = "size-[8px]";
const GAP = "gap-[3px]";
/** 月份轴高度，星期标签用它对齐首行 */
const AXIS_HEIGHT = "h-[12px]";

/**
 * GitHub 风格的历史热力图：一列一周，从上到下为周一 → 周日。
 * 点格子可以切换"正在查看的那一天"。
 */
export function Heatmap({
  data,
  selected,
  onSelect,
}: {
  data: HeatmapData;
  selected: string;
  onSelect: (date: string) => void;
}) {
  const { t, lang } = useI18n();
  const today = todayKey();

  // 补齐首周空白，使每列都是完整的 7 天
  const padded: (HeatmapCell | null)[] = [
    ...Array.from({ length: data.leadingBlanks }, () => null),
    ...data.cells,
  ];
  const columns: (HeatmapCell | null)[][] = [];
  for (let index = 0; index < padded.length; index += 7) {
    columns.push(padded.slice(index, index + 7));
  }

  // 月份标签：某列里出现「1 号」就把月份标在这一列。
  // 相邻标签至少间隔 3 列，避免"11月""12月"挤在一起。
  const monthLabels: string[] = [];
  let lastMonth = "";
  let lastLabelColumn = -3;
  columns.forEach((column, index) => {
    const firstOfMonth = column.find((cell) => cell && cell.date.endsWith("-01"));
    const month = (firstOfMonth ?? column.find(Boolean))?.date.slice(5, 7) ?? "";
    if (!month || month === lastMonth) return;
    if (index - lastLabelColumn < 3) return;
    lastMonth = month;
    lastLabelColumn = index;
    monthLabels[index] = lang === "zh" ? `${Number(month)}月` : MONTH_NAMES[Number(month) - 1];
  });

  return (
    <div className="flex gap-1.5">
      {/* 星期标签：只显示一、三、五、日 */}
      <div className={cn("flex flex-col pt-[16px] text-[8px] leading-none text-ink-faint", GAP)}>
        {["一", "二", "三", "四", "五", "六", "日"].map((label, index) => (
          <span key={label} className="flex h-[8px] w-2.5 items-center">
            {index % 2 === 0 ? label : ""}
          </span>
        ))}
      </div>

      <div className="min-w-0">
        {/* 月份轴 */}
        <div className={cn("mb-1 flex", AXIS_HEIGHT, GAP)}>
          {columns.map((_, index) => (
            <span
              key={index}
              className="w-[8px] shrink-0 whitespace-nowrap text-[8px] leading-[12px] text-ink-faint"
            >
              {monthLabels[index]}
            </span>
          ))}
        </div>

        <div className={cn("flex", GAP)}>
          {columns.map((column, columnIndex) => (
            <div key={columnIndex} className={cn("flex flex-col", GAP)}>
              {column.map((cell, rowIndex) =>
                cell ? (
                  <button
                    key={cell.date}
                    type="button"
                    onClick={() => onSelect(cell.date)}
                    title={`${formatDayLabel(cell.date, lang)} · ${
                      cell.activeMs > 0 ? formatDuration(cell.activeMs, lang) : t("common.empty")
                    }`}
                    className={cn(
                      "rounded-[2px] transition-[box-shadow,transform]",
                      CELL,
                      LEVEL_CLASS[cell.level] ?? "bg-heat-0",
                      "hover:ring-1 hover:ring-ink/30",
                      cell.date === selected && "ring-[1.5px] ring-ink/60",
                      cell.date === today && cell.date !== selected && "ring-1 ring-accent/70",
                    )}
                  />
                ) : (
                  <span key={`blank-${columnIndex}-${rowIndex}`} className={CELL} />
                ),
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** 图例：少 → 多 */
export function HeatmapLegend() {
  const { t } = useI18n();
  return (
    <div className="flex items-center gap-1 text-[10px] text-ink-faint">
      <span>{t("stats.legendLess")}</span>
      {LEVEL_CLASS.map((className) => (
        <span key={className} className={cn("size-[8px] rounded-[2px]", className)} />
      ))}
      <span>{t("stats.legendMore")}</span>
    </div>
  );
}
