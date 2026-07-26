"use client";

import { LogIn, PenLine, UserRound } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { Brand } from "@/components/brand";

export function ForumHeader() {
  const { status, user } = useAuth();

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
        </nav>
        <div className="ml-auto flex min-w-28 items-center justify-end gap-2">
          {status === "authenticated" ? (
            <>
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
