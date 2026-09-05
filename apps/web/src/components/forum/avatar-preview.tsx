"use client";

import type { ReactNode } from "react";
import { X } from "lucide-react";
import { useEffect, useState } from "react";

/**
 * Wraps an avatar thumbnail: clicking it opens a lightbox with the full-size
 * image. Falls back to the plain child when there is no image to show.
 */
export function AvatarPreview({
  src,
  alt,
  children,
}: {
  src?: string | null;
  alt: string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
    };
  }, [open]);

  if (!src) {
    return <>{children}</>;
  }

  return (
    <>
      <button
        type="button"
        className="cursor-zoom-in rounded-full focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => setOpen(true)}
        aria-label={`查看 ${alt} 的头像大图`}
        title="查看头像大图"
      >
        {children}
      </button>
      {open ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-6"
          onClick={() => setOpen(false)}
          role="dialog"
          aria-modal="true"
          aria-label={`${alt} 的头像大图`}
        >
          {/* External avatar URLs are user-provided; bypass Next image
              optimization exactly like the avatar thumbnails do. */}
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={src}
            alt={`${alt} 的头像`}
            className="max-h-[85vh] max-w-[85vw] rounded-lg object-contain shadow-2xl"
            onClick={(event) => event.stopPropagation()}
          />
          <button
            type="button"
            className="absolute right-4 top-4 inline-flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-white hover:bg-white/20"
            aria-label="关闭大图"
          >
            <X className="size-5" aria-hidden="true" />
          </button>
        </div>
      ) : null}
    </>
  );
}
