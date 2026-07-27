"use client";

import { useQuery } from "@tanstack/react-query";
import { Bell, Bookmark, LogIn, PenLine, UserRound } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { Brand } from "@/components/brand";
import { getUnreadCount, notificationKeys } from "@/lib/api/notifications";

export function ForumHeader() {
  const { status, user } = useAuth();
  const unread = useQuery({
    queryKey: notificationKeys.unread,
    queryFn: getUnreadCount,
    enabled: status === "authenticated",
    refetchInterval: 60_000,
    staleTime: 15_000,
  });
  const unreadCount = unread.data?.count ?? 0;

  return (
    <header className="sticky top-0 z-40 border-b border-border bg-white/95 backdrop-blur">
      <div className="mx-auto flex h-16 max-w-6xl items-center gap-6 px-5 sm:px-8">
        <Brand />
        <nav className="hidden items-center gap-5 text-sm sm:flex" aria-label="主导航">
          <Link href="/" className="text-muted-foreground hover:text-foreground">
            首页
          </Link>
          <Link href="/categories" className="text-muted-foreground hover:text-foreground">
            板块
          </Link>
          {status === "authenticated" ? (
            <>
              <Link href="/favorites" className="text-muted-foreground hover:text-foreground">
                收藏
              </Link>
              <Link href="/notifications" className="text-muted-foreground hover:text-foreground">
                通知
              </Link>
            </>
          ) : null}
        </nav>
        <div className="ml-auto flex min-w-28 items-center justify-end gap-2">
          {status === "authenticated" ? (
            <>
              <Link
                href="/notifications"
                className="relative inline-flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium hover:bg-muted"
                aria-label={unreadCount > 0 ? `通知，${unreadCount} 条未读` : "通知"}
              >
                <Bell className="size-4" aria-hidden="true" />
                {unreadCount > 0 ? (
                  <span className="absolute right-1 top-1 inline-flex min-w-4 items-center justify-center rounded-full bg-destructive px-1 text-[10px] font-semibold leading-4 text-white">
                    {unreadCount > 99 ? "99+" : unreadCount}
                  </span>
                ) : null}
              </Link>
              <Link
                href="/favorites"
                className="inline-flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium hover:bg-muted sm:hidden"
                aria-label="我的收藏"
              >
                <Bookmark className="size-4" aria-hidden="true" />
              </Link>
              <Link
                href="/topics/new"
                className="hidden h-9 items-center gap-2 rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90 sm:inline-flex"
              >
                <PenLine className="size-4" aria-hidden="true" />
                发帖
              </Link>
              <Link
                href="/profile"
                className="inline-flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium hover:bg-muted"
              >
                <UserRound className="size-4" aria-hidden="true" />
                <span className="max-w-24 truncate">{user?.nickname || user?.username}</span>
              </Link>
            </>
          ) : status === "unauthenticated" ? (
            <Link
              href="/login"
              className="inline-flex h-9 items-center gap-2 rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
            >
              <LogIn className="size-4" aria-hidden="true" />
              登录
            </Link>
          ) : (
            <span
              className="h-9 w-24 animate-pulse rounded-md bg-muted"
              aria-label="正在确认登录状态"
            />
          )}
        </div>
      </div>
    </header>
  );
}
