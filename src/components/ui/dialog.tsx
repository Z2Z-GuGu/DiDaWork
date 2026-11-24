import * as React from "react";
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";

import { cn } from "@/lib/utils";

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogClose = DialogPrimitive.Close;

export const DialogContent = React.forwardRef<
  React.ComponentRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content> & { label?: string }
>(({ className, children, label, ...props }, ref) => (
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay className="fixed inset-0 z-50 bg-black/25 backdrop-blur-[2px]" />
    <DialogPrimitive.Content
      ref={ref}
      className={cn(
        "fixed left-1/2 top-1/2 z-50 w-[420px] max-w-[calc(100vw-40px)] -translate-x-1/2 -translate-y-1/2",
        "rounded-2xl border border-hairline bg-surface p-5 shadow-float outline-none",
        "data-[state=open]:animate-rise",
        className,
      )}
      {...props}
    >
      {label ? (
        <DialogPrimitive.Title className="text-[15px] font-semibold text-ink">
          {label}
        </DialogPrimitive.Title>
      ) : null}
      {children}
      <DialogPrimitive.Close className="absolute right-4 top-4 rounded-md p-1 text-ink-faint transition-colors hover:bg-hairline hover:text-ink">
        <X className="size-4" />
      </DialogPrimitive.Close>
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
));
DialogContent.displayName = "DialogContent";
