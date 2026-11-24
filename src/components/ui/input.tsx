import * as React from "react";
import { cn } from "@/lib/utils";

export const Input = React.forwardRef<
  HTMLInputElement,
  React.InputHTMLAttributes<HTMLInputElement>
>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn(
      "h-8 w-full rounded-lg border border-hairline-strong bg-surface px-2.5 text-[13px] text-ink",
      "placeholder:text-ink-faint focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/25",
      "outline-none transition-colors disabled:opacity-50",
      className,
    )}
    {...props}
  />
));
Input.displayName = "Input";

const pad = (value: number) => `${value}`.padStart(2, "0");

/**
 * 把用户输入的任意内容规范成 `HH:MM`。
 * 支持 `930`、`9:3`、`18` 这类简写；无法解析时返回 null。
 */
function normalizeClock(raw: string): string | null {
  const digits = raw.replace(/\D/g, "");
  if (digits.length === 0) return null;
  const hour = Number(digits.length <= 2 ? digits : digits.slice(0, digits.length - 2));
  const minute = digits.length <= 2 ? 0 : Number(digits.slice(-2));
  if (Number.isNaN(hour) || Number.isNaN(minute) || hour > 23 || minute > 59) return null;
  return `${pad(hour)}:${pad(minute)}`;
}

/**
 * 24 小时制的时间输入。
 *
 * 没有用原生 `input[type=time]`：它在不同系统语言下会变成 12 小时制（`06:00 PM`），
 * 与界面上其它地方的 `18:00` 不一致，宽度也不好控制。
 * 这里用文本框 + 失焦时规范化，跨平台表现完全一致。
 */
export function TimeInput({
  value,
  onValueChange,
  className,
  disabled,
}: {
  value: string;
  onValueChange: (next: string) => void;
  className?: string;
  disabled?: boolean;
}) {
  const [text, setText] = React.useState(value);

  // 外部值变化（例如切换编辑的闹钟）时同步
  React.useEffect(() => setText(value), [value]);

  const commit = () => {
    const normalized = normalizeClock(text);
    if (!normalized) {
      setText(value); // 解析失败就恢复成原来的合法值
      return;
    }
    setText(normalized);
    if (normalized !== value) onValueChange(normalized);
  };

  return (
    <input
      value={text}
      disabled={disabled}
      inputMode="numeric"
      maxLength={5}
      placeholder="--:--"
      onChange={(event) => setText(event.target.value.replace(/[^\d:]/g, "").slice(0, 5))}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          commit();
          event.currentTarget.blur();
        }
      }}
      className={cn(
        "tabular h-9 w-[88px] rounded-lg border border-hairline-strong bg-surface px-2 text-center",
        "text-[15px] font-medium text-ink placeholder:text-ink-faint",
        "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent/25",
        "outline-none transition-colors disabled:opacity-50",
        className,
      )}
    />
  );
}
