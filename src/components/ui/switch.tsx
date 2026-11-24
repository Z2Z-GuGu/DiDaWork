import * as React from "react";
import * as SwitchPrimitive from "@radix-ui/react-switch";

import { cn } from "@/lib/utils";

export const Switch = React.forwardRef<
  React.ComponentRef<typeof SwitchPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof SwitchPrimitive.Root>
>(({ className, ...props }, ref) => (
  <SwitchPrimitive.Root
    ref={ref}
    className={cn(
      "peer inline-flex h-[20px] w-[34px] shrink-0 cursor-pointer items-center rounded-full border border-transparent transition-colors outline-none",
      "data-[state=checked]:bg-accent data-[state=unchecked]:bg-hairline-strong",
      "focus-visible:ring-2 focus-visible:ring-accent/40 disabled:cursor-not-allowed disabled:opacity-50",
      className,
    )}
    {...props}
  >
    <SwitchPrimitive.Thumb
      className={cn(
        "pointer-events-none block h-[16px] w-[16px] rounded-full bg-white shadow-sm transition-transform",
        "data-[state=checked]:translate-x-[16px] data-[state=unchecked]:translate-x-[2px]",
        "data-[state=checked]:shadow-[0_1px_2px_rgba(0,0,0,0.25)]",
      )}
    />
  </SwitchPrimitive.Root>
));
Switch.displayName = "Switch";
