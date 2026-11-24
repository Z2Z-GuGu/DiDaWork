import { useEffect, useMemo, useState } from "react";
import { ChevronLeft } from "lucide-react";

import { useAppData } from "@/app/AppDataProvider";
import { useNow } from "@/hooks/useNow";
import { Heatmap, HeatmapLegend } from "@/components/charts/Heatmap";
import { TimelineRows } from "@/components/charts/TimelineRows";
import { TrendChart } from "@/components/charts/TrendChart";
import { PageHeader } from "@/components/layout/PageHeader";
import { Section } from "@/components/layout/Section";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/i18n";
import { api, toMessage } from "@/lib/api";
import {
  formatClock,
  formatDayLabel,
  formatDuration,
  todayKey,
} from "@/lib/format";
import { cn } from "@/lib/utils";
import type { DaySummary, Heatmap as HeatmapData, TimelineDay, Trend } from "@/lib/types";

const TREND_DAYS = 14;
const TIMELINE_DAYS = 7;
const HEATMAP_WEEKS = 53;

/**
 * 统计页。
 *
 * 页面上所有卡片都围绕"当前查看的那一天"（selected）联动：
 * 点热力图格子 / 时间段的某一行，都会把 selected 切过去，四块数据一起重算。
 */
export function DashboardPage() {
  const { t, lang } = useI18n();
  const { sampleTick } = useAppData();

  const today = todayKey();
  const [selected, setSelected] = useState(today);
  const [summary, setSummary] = useState<DaySummary | null>(null);
  const [trend, setTrend] = useState<Trend | null>(null);
  const [timeline, setTimeline] = useState<TimelineDay[]>([]);
  const [heatmap, setHeatmap] = useState<HeatmapData | null>(null);
  const [error, setError] = useState<string | null>(null);
  // 用于"已持续 xx"的滚动计时，不需要和后端同步
  const now = useNow();

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const [nextSummary, nextTrend, nextTimeline, nextHeatmap] = await Promise.all([
          api.getDaySummary(selected),
          api.getTrend(TREND_DAYS, selected),
          api.getTimeline(TIMELINE_DAYS, selected),
          api.getHeatmap(HEATMAP_WEEKS),
        ]);
        if (cancelled) return;
        setSummary(nextSummary);
        setTrend(nextTrend);
        setTimeline(nextTimeline);
        setHeatmap(nextHeatmap);
        setError(null);
      } catch (err) {
        if (!cancelled) setError(toMessage(err));
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [selected, sampleTick]);

  const sessionLabel = useMemo(() => {
    if (!summary || summary.activeMs === 0) return t("stats.dayEmpty");
    if (summary.live) {
      const last = summary.segments[summary.segments.length - 1];
      if (!last) return t("stats.dayEmpty");
      return t("stats.liveSince", {
        duration: formatDuration(Math.max(0, now - last.startMs), lang),
      });
    }
    if (summary.firstStartMs && summary.lastEndMs) {
      return `${formatClock(summary.firstStartMs)} – ${formatClock(summary.lastEndMs)}`;
    }
    return t("stats.dayEmpty");
  }, [summary, now, lang, t]);

  const metrics = [
    {
      label: t("stats.firstStart"),
      value: summary?.firstStartMs ? formatClock(summary.firstStartMs) : "—",
    },
    {
      label: t("stats.lastEnd"),
      value: summary?.lastEndMs ? formatClock(summary.lastEndMs) : "—",
    },
    {
      label: t("stats.longest"),
      value: summary ? formatDuration(summary.longestMs, lang) : "—",
    },
    {
      label: t("stats.sessions"),
      value: summary ? `${summary.segments.length}` : "—",
    },
  ];

  return (
    <>
      <PageHeader
        title={t("stats.title")}
        subtitle={t("stats.sampleHint")}
        actions={
          selected !== today ? (
            <Button variant="secondary" size="sm" onClick={() => setSelected(today)}>
              <ChevronLeft className="size-3.5" />
              {t("stats.backToToday")}
            </Button>
          ) : null
        }
      />

      {error ? (
        <div className="mb-3 rounded-lg border border-danger/30 bg-danger/5 px-3 py-2 text-[12px] text-danger">
          {error}
        </div>
      ) : null}

      {/* 当日总览 */}
      <Section className="mb-3">
        <div className="flex items-start justify-between gap-8 py-1">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-[12px] text-ink-soft">
              <span>{formatDayLabel(selected, lang)}</span>
              {selected === today && (
                <span className="rounded-full bg-accent-soft px-1.5 py-[1px] text-[10px] font-medium text-accent">
                  {t("common.today")}
                </span>
              )}
              {summary?.live && (
                <span className="flex items-center gap-1 rounded-full bg-accent-soft px-1.5 py-[1px] text-[10px] font-medium text-accent">
                  <span className="size-1.5 rounded-full bg-accent" />
                  {t("stats.live")}
                </span>
              )}
            </div>

            <div className="mt-1.5 flex items-baseline gap-1.5">
              <span className="tabular text-[34px] font-semibold leading-none tracking-tight text-ink">
                {summary ? formatDuration(summary.activeMs, lang) : "—"}
              </span>
            </div>

            <p className="mt-2 text-[11px] text-ink-faint">{sessionLabel}</p>
          </div>

          <div className="grid shrink-0 grid-cols-2 gap-x-7 gap-y-3 text-right">
            {metrics.map((metric) => (
              <div key={metric.label}>
                <div className="text-[10px] text-ink-faint">{metric.label}</div>
                <div className="tabular mt-0.5 text-[13px] font-medium text-ink">
                  {metric.value}
                </div>
              </div>
            ))}
          </div>
        </div>
      </Section>

      {/* 近半月趋势 */}
      <Section
        className="mb-3"
        title={t("stats.trend")}
        hint={
          trend
            ? t("stats.trendHint", {
                days: TREND_DAYS,
                total: formatDuration(trend.totalMs, lang),
                avg: formatDuration(trend.averageMs, lang),
              })
            : undefined
        }
      >
        {trend ? (
          <TrendChart
            points={trend.points}
            averageMs={trend.averageMs}
            maxMs={trend.maxMs}
            selected={selected}
            onSelect={setSelected}
          />
        ) : (
          <Skeleton height={112} />
        )}
      </Section>

      {/* 近 7 天时间段 */}
      <Section className="mb-3" title={t("stats.timeline")} hint={t("stats.timelineHint")}>
        {timeline.length > 0 ? (
          <TimelineRows days={timeline} selected={selected} onSelect={setSelected} />
        ) : (
          <Skeleton height={120} />
        )}
      </Section>

      {/* 历史热力图 */}
      <Section
        title={t("stats.heatmap")}
        hint={t("stats.heatmapHint")}
        actions={<HeatmapLegend />}
      >
        {heatmap ? (
          <div className="overflow-x-auto pb-1">
            <Heatmap data={heatmap} selected={selected} onSelect={setSelected} />
          </div>
        ) : (
          <Skeleton height={110} />
        )}
      </Section>
    </>
  );
}

function Skeleton({ height }: { height: number }) {
  return (
    <div
      className={cn("w-full animate-pulse rounded-lg bg-heat-0/70")}
      style={{ height }}
    />
  );
}
