import * as React from "react";

import { cn } from "../lib/cn";

export function Avatar({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "relative flex size-12 shrink-0 overflow-hidden rounded-full bg-muted",
        className,
      )}
      {...props}
    />
  );
}

export function AvatarImage({
  className,
  alt = "",
  ...props
}: React.ImgHTMLAttributes<HTMLImageElement>) {
  return (
    // External avatar URLs are user-provided and intentionally bypass Next image optimization.
    <img
      className={cn("absolute inset-0 z-10 size-full object-cover", className)}
      alt={alt}
      {...props}
    />
  );
}

export function AvatarFallback({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn("flex size-full items-center justify-center text-sm font-semibold", className)}
      {...props}
    />
  );
}
