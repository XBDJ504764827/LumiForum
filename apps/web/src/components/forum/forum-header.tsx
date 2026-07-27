"use client";

import type { Route } from "next";
import { useQuery } from "@tanstack/react-query";
import { Bell, Bookmark, LogIn, PenLine, Search, UserRound } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { Brand } from "@/components/brand";
import { getUnreadCount, notificationKeys } from "@/lib/api/notifications";
import { saveRecentSearch } from "@/lib/api/search";

export function ForumHeader() {
  const router = useRouter();
  const { status, user } = useAuth();
  const [q, setQ] = useState("");
  const unread = useQuery({
    queryKey: notificationKeys.unread,
    queryFn: getUnreadCount,
    enabled: status === "authenticated",
    refetchInterval: 60_000,
    staleTime: 15_000,
  });
  const unreadCount = unread.data?.count ?? 0;

  const onSearch = (event: FormEvent) => {
    event.preventDefault();
    const value = q.trim();
    if (!value) {
      router.push("/search" as Route);
      return;
    }
    saveRecentSearch(value);
    router.push(`/search?q=${encodeURIComponent(value)}` as Route);
  };

  return (
    <header className="sticky top-0 z-40 border-b border-border bg-white/95 backdrop-blur">
      <div className="mx-auto flex h-16 max-w-6xl items-center gap-4 px-5 sm:gap-6 sm:px-8">
        <Brand />
        <nav className="hidden items-center gap-5 text-sm md:flex" aria-label="主导航">
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
              {user?.role.code === "administrator" || user?.role.code === "super_administrator" ? (
                <Link href="/admin" className="text-muted-foreground hover:text-foreground">
                  后台
                </Link>
              ) : null}
            </>
          ) : null}
        </nav>

        <form
          onSubmit={onSearch}
          className="ml-auto hidden min-w-0 max-w-xs flex-1 items-center gap-2 lg:flex"
        >
          <label className="relative block w-full">
            <span className="sr-only">搜索</span>
            <Search
              className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
              aria-hidden="true"
            />
            <input
              value={q}
              onChange={(event) => setQ(event.target.value)}
              placeholder="搜索帖子 / 用户"
              className="h-9 w-full rounded-md border border-border bg-white pl-9 pr-3 text-sm outline-none ring-offset-background placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring"
            />
          </label>
        </form>

        <div className="flex min-w-28 items-center justify-end gap-2 lg:ml-0">
          <Link
            href="/search"
            className="inline-flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium hover:bg-muted lg:hidden"
            aria-label="搜索"
          >
            <Search className="size-4" aria-hidden="true" />
          </Link>
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
