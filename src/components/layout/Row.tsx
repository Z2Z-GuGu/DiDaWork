import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * 设置项行：左侧标题 + 说明，右侧控件。
 * 多行之间用 `divide-y` 分隔，保持 macOS 系统设置的观感。
 */
export function Row({
  title,
  hint,
  children,
  className,
  stacked = false,
}: {
  title: ReactNode;
  hint?: ReactNode;
  children?: ReactNode;
  className?: string;
  /** 控件换行显示（用于分段选择器之类较宽的控件） */
  stacked?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex gap-6 py-2.5",
        stacked ? "flex-col items-start gap-2" : "items-center justify-between",
        className,
      )}
    >
      <div className="min-w-0">
        <div className="text-[13px] leading-snug text-ink">{title}</div>
        {hint ? (
          <div className="mt-0.5 text-[11px] leading-relaxed text-ink-faint">{hint}</div>
        ) : null}
      </div>
      {children ? (
        <div className={cn("shrink-0", stacked && "w-full")}>{children}</div>
      ) : null}
    </div>
  );
}
