"use client";

import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { cn } from "@lumiforum/ui";

/**
 * Non-blocking banner shown when a silent token refresh fails (the session
 * expired while the user was browsing). Dismissible; links to /login so the
 * user can re-authenticate without losing the current page.
 */
export function SessionExpiredBanner() {
  const { sessionExpired, dismissSessionExpired } = useAuth();

  if (!sessionExpired) return null;

  return (
    <div
      role="alert"
      className={cn(
        "fixed inset-x-0 top-16 z-50 mx-auto flex max-w-6xl items-center justify-between gap-4",
        "border-b border-border bg-destructive/10 px-5 py-3 text-sm text-destructive backdrop-blur",
      )}
    >
      <span>登录状态已过期，请重新登录后继续操作。</span>
      <span className="flex shrink-0 items-center gap-3">
        <Link
          href="/login"
          className="rounded-md bg-destructive px-3 py-1.5 font-medium text-destructive-foreground hover:bg-destructive/90"
        >
          去登录
        </Link>
        <button
          type="button"
          onClick={dismissSessionExpired}
          className="text-muted-foreground underline-offset-4 hover:underline"
          aria-label="关闭提示"
        >
          关闭
        </button>
      </span>
    </div>
  );
}
