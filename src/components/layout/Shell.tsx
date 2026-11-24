import type { ReactNode } from "react";

import { useAppData } from "@/app/AppDataProvider";
import { cn } from "@/lib/utils";
import { NAV_ITEMS, type Tab } from "@/app/navigation";
import { useI18n } from "@/i18n";
import { formatDuration } from "@/lib/format";
import { RingBanner } from "@/features/ring/RingBanner";
import { WindowControls } from "./WindowControls";


/**
 * 主窗口骨架：左侧导航 + 右侧内容。
 *
 * 窗口没有系统标题栏：
 * - macOS 用 `titleBarStyle: overlay`，内容铺满整窗，红黄绿三个交通灯浮在左上角，
 *   所以侧边栏顶部留出一条可拖拽的空白给它；
 * - Windows / Linux 关掉了原生装饰，右侧自绘最小化 / 最大化 / 关闭。
 *
 * 两种平台顶部预留的高度一致，内容起点对齐。
 */
const IS_MAC = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent);
const TITLE_BAR_HEIGHT = "h-11";

export function Shell({ tab, onTabChange, children }: {
  tab: Tab;
  onTabChange: (tab: Tab) => void;
  children: ReactNode;
}) {
  return (
    <div className="relative flex h-full w-full overflow-hidden bg-canvas">
      <Sidebar tab={tab} onTabChange={onTabChange} />
      <main className="flex min-w-0 flex-1 flex-col">
        <div
          data-tauri-drag-region
          className={cn("drag-region flex shrink-0 items-center justify-end", TITLE_BAR_HEIGHT)}
        >
          {IS_MAC ? null : <WindowControls />}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto px-7 pb-8">
          <RingBanner />
          {children}
        </div>
      </main>

      {/* 没有系统标题栏，就用一行静态标题表明这是哪个应用。
          压在最上层但 pointer-events-none，拖拽与右侧按钮都不受影响。 */}
      <div
        className={cn(
          "pointer-events-none absolute inset-x-0 top-0 flex items-center justify-center",
          TITLE_BAR_HEIGHT,
        )}
      >
        <span className="text-[13px] font-semibold tracking-[0.01em] text-ink">DiDaWork</span>
      </div>
    </div>
  );
}

function Sidebar({ tab, onTabChange }: { tab: Tab; onTabChange: (tab: Tab) => void }) {
  const { t } = useI18n();

  return (
    <aside className="flex w-[188px] shrink-0 flex-col border-r border-hairline bg-sidebar">
      {/* 交通灯 / 拖拽区：不要放控件，否则会被盖住 */}
      <div data-tauri-drag-region className={cn("drag-region shrink-0", TITLE_BAR_HEIGHT)} />

      <nav className="flex flex-col gap-0.5 px-2">
        {NAV_ITEMS.map((item) => {
          const active = item.key === tab;
          const Icon = item.icon;
          return (
            <button
              key={item.key}
              onClick={() => onTabChange(item.key)}
              className={cn(
                "flex h-[30px] items-center gap-2 rounded-lg px-2.5 text-[13px] transition-colors",
                active
                  ? "bg-accent-soft font-medium text-ink"
                  : "text-ink-soft hover:bg-hairline/60",
              )}
            >
              <Icon
                className={cn("size-[15px]", active ? "text-accent" : "text-ink-faint")}
                strokeWidth={2}
              />
              {t(item.labelKey)}
            </button>
          );
        })}
      </nav>

      <LiveBadge />
    </aside>
  );
}

/** 侧边栏底部的实时状态，让用户随时知道有没有在记录。 */
function LiveBadge() {
  const { t, lang } = useI18n();
  const { live } = useAppData();

  return (
    <div className="mt-auto px-3 pb-3">
      <div className="flex items-center gap-2 rounded-xl border border-hairline bg-canvas/70 px-2.5 py-2">
        <span className="relative flex size-2 items-center justify-center">
          <span
            className={cn(
              "absolute inline-flex size-2 rounded-full",
              live.isActive ? "bg-accent" : "bg-ink-faint/60",
            )}
          />
          {live.isActive && (
            <span className="absolute inline-flex size-2 animate-ping rounded-full bg-accent/60" />
          )}
        </span>
        <div className="min-w-0 leading-tight">
          <div className="truncate text-[11px] text-ink-soft">
            {live.isActive ? t("stats.live") : t("stats.idle")}
          </div>
          <div className="tabular text-[11px] font-medium text-ink">
            {formatDuration(live.todayActiveMs, lang)}
          </div>
        </div>
      </div>
    </div>
  );
}
