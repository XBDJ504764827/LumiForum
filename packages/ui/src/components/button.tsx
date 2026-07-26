import * as React from "react";

import { cn } from "../lib/cn";

export type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "outline" | "ghost";
  size?: "sm" | "md" | "lg";
};

const variantClass: Record<NonNullable<ButtonProps["variant"]>, string> = {
  default: "bg-zinc-900 text-zinc-50 hover:bg-zinc-800",
  outline: "border border-zinc-300 bg-transparent hover:bg-zinc-100",
  ghost: "bg-transparent hover:bg-zinc-100",
};

const sizeClass: Record<NonNullable<ButtonProps["size"]>, string> = {
  sm: "h-8 px-3 text-xs",
  md: "h-10 px-4 text-sm",
  lg: "h-11 px-6 text-base",
};

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = "default", size = "md", type = "button", ...props }, ref) => {
    return (
      <button
        ref={ref}
        type={type}
        className={cn(
          "inline-flex items-center justify-center rounded-md font-medium transition-colors",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400",
          "disabled:pointer-events-none disabled:opacity-50",
          variantClass[variant],
          sizeClass[size],
          className,
        )}
        {...props}
      />
    );
  },
);

Button.displayName = "Button";
