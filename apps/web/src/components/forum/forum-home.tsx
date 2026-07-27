"use client";

import { useQuery } from "@tanstack/react-query";
import { Flame, Pin } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { CategoryNav } from "@/components/forum/category-nav";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicList } from "@/components/forum/topic-list";
import { forumKeys, listCategories, listTopics } from "@/lib/api/forum";

export function ForumHome() {
  const { status } = useAuth();
  const categories = useQuery({
    queryKey: forumKeys.categories,
    queryFn: listCategories,
  });
  const pinnedParams = { sort: "pinned" as const, page: 1, page_size: 5 };
  const latestParams = { sort: "latest" as const, page: 1, page_size: 20 };
  const pinned = useQuery({
    queryKey: forumKeys.topics(pinnedParams),
    queryFn: () => listTopics(pinnedParams),
  });
  const latest = useQuery({
    queryKey: forumKeys.topics(latestParams),
    queryFn: () => listTopics(latestParams),
  });

  return (
    <main className="mx-auto max-w-6xl px-5 py-9 sm:px-8">
      <div className="mb-8 flex items-end justify-between gap-5 border-b border-border pb-7">
        <div>
          <p className="text-sm font-medium text-primary">社区动态</p>
          <h1 className="mt-2 text-3xl font-semibold">最新讨论</h1>
        </div>
        <Link
          href={status === "authenticated" ? "/topics/new" : "/login"}
          className="inline-flex h-10 items-center rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          发布帖子
        </Link>
      </div>

      <div className="grid gap-10 lg:grid-cols-[220px_minmax(0,1fr)]">
        <aside className="lg:border-r lg:border-border lg:pr-7">
          {categories.isPending ? (
            <QueryLoading label="正在加载板块" />
          ) : categories.isError ? (
            <QueryError />
          ) : (
            <CategoryNav categories={categories.data ?? []} />
          )}
        </aside>

        <div className="min-w-0">
          {pinned.data && pinned.data.items.length > 0 ? (
            <section className="mb-10" aria-labelledby="pinned-title">
              <h2 id="pinned-title" className="mb-4 flex items-center gap-2 text-sm font-semibold">
                <Pin className="size-4" aria-hidden="true" />
                置顶主题
              </h2>
              <TopicList topics={pinned.data.items} />
            </section>
          ) : null}

          <section aria-labelledby="latest-title">
            <h2 id="latest-title" className="mb-4 flex items-center gap-2 text-sm font-semibold">
              <Flame className="size-4 text-accent" aria-hidden="true" />
              最新发布
            </h2>
            {latest.isPending ? (
              <QueryLoading label="正在加载帖子" />
            ) : latest.isError ? (
              <QueryError />
            ) : (
              <TopicList topics={latest.data?.items ?? []} />
            )}
          </section>
        </div>
      </div>
    </main>
  );
}
