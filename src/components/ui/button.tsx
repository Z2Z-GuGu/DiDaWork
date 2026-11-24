import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-lg font-medium transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-40 focus-visible:ring-2 focus-visible:ring-accent/40 active:scale-[0.985]",
  {
    variants: {
      variant: {
        primary: "bg-accent text-on-accent shadow-sm hover:bg-accent-hover",
        secondary: "bg-subtle text-ink border border-hairline hover:bg-hairline/60",
        ghost: "text-ink-soft hover:bg-hairline/70 hover:text-ink",
        outline: "border border-hairline-strong text-ink hover:bg-subtle",
        danger: "text-danger hover:bg-danger/10",
        plain: "",
      },
      size: {
        sm: "h-7 px-2.5 text-xs",
        md: "h-8 px-3.5 text-[13px]",
        lg: "h-9 px-4 text-[13px]",
        icon: "h-7 w-7",
        "icon-sm": "h-6 w-6",
      },
    },
    defaultVariants: { variant: "secondary", size: "md" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild, type = "button", ...props }, ref) => {
    const Component = asChild ? Slot : "button";
    return (
      <Component
        ref={ref}
        type={asChild ? undefined : type}
        className={cn(buttonVariants({ variant, size }), className)}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";
