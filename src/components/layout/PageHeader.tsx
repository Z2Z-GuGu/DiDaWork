import type { ReactNode } from "react";

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="flex items-end justify-between gap-4 pb-4 pt-1">
      <div className="min-w-0">
        <h1 className="text-[19px] font-semibold tracking-[-0.01em] text-ink">{title}</h1>
        {subtitle ? (
          <div className="mt-1 text-[12px] leading-relaxed text-ink-soft">{subtitle}</div>
        ) : null}
      </div>
      {actions ? <div className="no-drag flex shrink-0 items-center gap-2">{actions}</div> : null}
    </header>
  );
}
