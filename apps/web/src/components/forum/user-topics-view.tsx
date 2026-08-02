"use client";

import type { Route } from "next";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, FileText } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicList } from "@/components/forum/topic-list";
import { forumKeys, listTopics } from "@/lib/api/forum";

const PAGE_SIZE = 20;

export function UserTopicsView({ userId }: { userId: string }) {
  const searchParams = useSearchParams();
  const page = Math.max(1, Number(searchParams.get("page") ?? "1") || 1);
  const params = { author_id: userId, sort: "latest" as const, page, page_size: PAGE_SIZE };
  const topics = useQuery({
    queryKey: forumKeys.topics(params),
    queryFn: () => listTopics(params),
  });

  return (
    <main className="mx-auto max-w-5xl px-5 py-9 sm:px-8">
      <div className="mb-7 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <Link href={`/users/${encodeURIComponent(userId)}/followers`} className="hover:text-foreground">
            用户
          </Link>
          <span>/</span>
          <span>发布的帖子</span>
        </div>
        <h1 className="mt-3 flex items-center gap-2 text-3xl font-semibold">
          <FileText className="size-7 text-primary" aria-hidden="true" />
          发布的帖子
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          该用户发布的全部帖子，按发布时间排序
        </p>
      </div>

      {topics.isPending ? (
        <QueryLoading label="正在加载帖子" />
      ) : topics.isError ? (
        <QueryError message="帖子加载失败" />
      ) : topics.data.items.length === 0 ? (
        <div className="border-y border-border py-14 text-center text-sm text-muted-foreground">
          还没有发布过帖子
        </div>
      ) : (
        <>
          <TopicList topics={topics.data.items} />
          <nav className="mt-7 flex items-center justify-between text-sm" aria-label="分页">
            <PageLink
              href={pageUrl(userId, page - 1)}
              disabled={page <= 1}
              label="上一页"
              icon={<ChevronLeft className="size-4" />}
            />
            <span className="text-muted-foreground">
              第 {topics.data.pagination.page} /{" "}
              {Math.max(1, topics.data.pagination.total_pages)} 页 · 共{" "}
              {topics.data.pagination.total} 个帖子
            </span>
            <PageLink
              href={pageUrl(userId, page + 1)}
              disabled={page >= topics.data.pagination.total_pages}
              label="下一页"
              icon={<ChevronRight className="size-4" />}
              iconAfter
            />
          </nav>
        </>
      )}
    </main>
  );
}

function pageUrl(userId: string, page: number): Route {
  return `/users/${encodeURIComponent(userId)}/topics?page=${Math.max(1, page)}` as Route;
}

function PageLink({
  href,
  disabled,
  label,
  icon,
  iconAfter = false,
}: {
  href: Route;
  disabled: boolean;
  label: string;
  icon: React.ReactNode;
  iconAfter?: boolean;
}) {
  if (disabled) {
    return (
      <span className="inline-flex h-9 items-center gap-1 px-3 text-muted-foreground/50">
        {label}
      </span>
    );
  }
  return (
    <Link href={href} className="inline-flex h-9 items-center gap-1 rounded-md px-3 hover:bg-muted">
      {!iconAfter ? icon : null}
      {label}
      {iconAfter ? icon : null}
    </Link>
  );
}
