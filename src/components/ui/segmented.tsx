import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
  icon?: ReactNode;
}

/**
 * 分段选择器。
 *
 * 用于选项少、需要一眼看全的场景（例如外观的三种模式），
 * 比下拉菜单少一次点击，也更接近 macOS 系统设置的观感。
 */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  className,
}: {
  value: T;
  options: Array<SegmentedOption<T>>;
  onChange: (value: T) => void;
  className?: string;
}) {
  return (
    <div
      role="radiogroup"
      className={cn(
        "inline-flex items-center gap-0.5 rounded-lg border border-hairline bg-subtle p-0.5",
        className,
      )}
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(option.value)}
            className={cn(
              "flex h-7 items-center gap-1.5 rounded-[6px] px-2.5 text-[12px] transition-colors",
              active
                ? "bg-accent-soft font-medium text-ink"
                : "text-ink-soft hover:bg-hairline/50 hover:text-ink",
            )}
          >
            {option.icon ? (
              <span className={cn(active ? "text-accent" : "text-ink-faint")}>{option.icon}</span>
            ) : null}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
