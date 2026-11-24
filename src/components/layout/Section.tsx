import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

/** 内容卡片：统一的圆角、描边与阴影。 */
export function Section({
  title,
  hint,
  actions,
  children,
  className,
  bodyClassName,
}: {
  title?: ReactNode;
  hint?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  bodyClassName?: string;
}) {
  const hasHeader = Boolean(title || hint || actions);
  return (
    <section
      className={cn(
        "rounded-card border border-hairline bg-surface shadow-card",
        className,
      )}
    >
      {hasHeader ? (
        <header className="flex items-start justify-between gap-3 px-4 pt-3.5">
          <div className="min-w-0">
            {title ? (
              <h2 className="text-[13px] font-semibold text-ink">{title}</h2>
            ) : null}
            {hint ? (
              <p className="mt-0.5 text-[11px] leading-relaxed text-ink-faint">{hint}</p>
            ) : null}
          </div>
          {actions ? <div className="shrink-0">{actions}</div> : null}
        </header>
      ) : null}
      <div className={cn("px-4 pb-4", hasHeader ? "pt-2.5" : "pt-4", bodyClassName)}>
        {children}
      </div>
    </section>
  );
}
