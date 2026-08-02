"use client";

import type { Route } from "next";
import {
  BarChart3,
  FileText,
  Files,
  Flag,
  FolderTree,
  LayoutDashboard,
  LogOut,
  MessageSquare,
  ScrollText,
  Users,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { Brand } from "@/components/brand";
import { cn } from "@lumiforum/ui";

const nav = [
  { href: "/admin", label: "仪表盘", icon: LayoutDashboard },
  { href: "/admin/users", label: "用户", icon: Users },
  { href: "/admin/topics", label: "帖子", icon: FileText },
  { href: "/admin/comments", label: "评论", icon: MessageSquare },
  { href: "/admin/categories", label: "分类", icon: FolderTree },
  { href: "/admin/files", label: "文件", icon: Files },
  { href: "/admin/polls", label: "投票", icon: BarChart3 },
  { href: "/admin/reports", label: "举报", icon: Flag },
  { href: "/admin/logs", label: "日志", icon: ScrollText },
] as const;

export function AdminShell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const { user, signOut } = useAuth();

  return (
    <div className="min-h-screen bg-surface">
      <div className="mx-auto grid min-h-screen max-w-[1400px] lg:grid-cols-[240px_minmax(0,1fr)]">
        <aside className="border-b border-border bg-white lg:border-b-0 lg:border-r">
          <div className="flex h-16 items-center px-5">
            <Brand />
          </div>
          <nav
            className="flex gap-1 overflow-x-auto px-3 pb-3 lg:flex-col lg:overflow-visible"
            aria-label="后台导航"
          >
            {nav.map((item) => {
              const active =
                item.href === "/admin" ? pathname === item.href : pathname.startsWith(item.href);
              const Icon = item.icon;
              return (
                <Link
                  key={item.href}
                  href={item.href as Route}
                  className={cn(
                    "inline-flex shrink-0 items-center gap-2 rounded-md px-3 py-2 text-sm",
                    active
                      ? "bg-muted font-medium text-foreground"
                      : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                  )}
                >
                  <Icon className="size-4" aria-hidden="true" />
                  {item.label}
                </Link>
              );
            })}
          </nav>
        </aside>

        <div className="min-w-0">
          <header className="flex h-16 items-center justify-between gap-4 border-b border-border bg-white px-5 sm:px-8">
            <div>
              <p className="text-sm text-muted-foreground">管理后台</p>
              <p className="text-sm font-medium">{user?.nickname || user?.username}</p>
            </div>
            <div className="flex items-center gap-3 text-sm">
              <Link href="/" className="text-muted-foreground hover:text-foreground">
                返回论坛
              </Link>
              <button
                type="button"
                className="inline-flex items-center gap-1.5 text-muted-foreground hover:text-foreground"
                onClick={() => void signOut()}
              >
                <LogOut className="size-4" aria-hidden="true" />
                退出
              </button>
            </div>
          </header>
          <main className="px-5 py-6 sm:px-8">{children}</main>
        </div>
      </div>
    </div>
  );
}
