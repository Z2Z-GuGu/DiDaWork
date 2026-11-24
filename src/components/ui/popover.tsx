import * as React from "react";
import * as PopoverPrimitive from "@radix-ui/react-popover";

import { cn } from "@/lib/utils";

/**
 * 轻量浮层。
 *
 * 比 `Select` 更适合"列表项里带操作"的场景：原生的 select 选项里放不了按钮，
 * 铃声列表需要在右对齐的位置放一个"查看版权"的图标。
 */
export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;

export const PopoverContent = React.forwardRef<
  React.ComponentRef<typeof PopoverPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof PopoverPrimitive.Content>
>(({ className, align = "end", sideOffset = 6, ...props }, ref) => (
  <PopoverPrimitive.Portal>
    <PopoverPrimitive.Content
      ref={ref}
      align={align}
      sideOffset={sideOffset}
      // 闹钟编辑本身是 Dialog（z-50），浮层要盖在它上面
      className={cn(
        "z-[60] rounded-xl border border-hairline bg-surface p-1 shadow-float outline-none",
        "data-[state=open]:animate-rise",
        className,
      )}
      {...props}
    />
  </PopoverPrimitive.Portal>
));
PopoverContent.displayName = "PopoverContent";
