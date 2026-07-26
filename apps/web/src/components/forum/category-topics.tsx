"use client";

import type { Route } from "next";
import type { TopicSort } from "@lumiforum/types";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight } from "lucide-react";
import Link from "next/link";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicList } from "@/components/forum/topic-list";
import { forumKeys, getCategory, listTopics } from "@/lib/api/forum";

const sorts: Array<{ value: TopicSort; label: string }> = [
  { value: "latest", label: "最新" },
  { value: "hot", label: "热门" },
  { value: "featured", label: "精华" },
  { value: "pinned", label: "置顶" },
];

export function CategoryTopics({
  slug,
  sort,
  page,
}: {
  slug: string;
  sort: TopicSort;
  page: number;
}) {
  const category = useQuery({
    queryKey: forumKeys.category(slug),
    queryFn: () => getCategory(slug),
  });
  const params = { category: slug, sort, page, page_size: 20 };
  const topics = useQuery({
    queryKey: forumKeys.topics(params),
    queryFn: () => listTopics(params),
  });

  if (category.isPending) return <QueryLoading label="正在加载板块" />;
  if (category.isError || !category.data) return <QueryError message="板块不存在或不可见" />;

  return (
    <main className="mx-auto max-w-5xl px-5 py-10 sm:px-8">
      <div className="mb-7 border-b border-border pb-7">
        <Link href="/categories" className="text-sm text-primary hover:underline">
          全部板块
        </Link>
        <div className="mt-3 flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
          <div>
            <h1 className="text-3xl font-semibold">{category.data.name}</h1>
            {category.data.description ? (
              <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
                {category.data.description}
              </p>
            ) : null}
          </div>
          <span className="text-sm text-muted-foreground">{category.data.topic_count} 个帖子</span>
        </div>
      </div>

      <div className="mb-5 flex flex-wrap items-center gap-1 border-b border-border" role="tablist">
        {sorts.map((item) => (
          <Link
            key={item.value}
            href={categoryUrl(slug, item.value, 1)}
            role="tab"
            aria-selected={sort === item.value}
            className={`border-b-2 px-3 py-2 text-sm font-medium ${
              sort === item.value
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {item.label}
          </Link>
        ))}
      </div>

      {topics.isPending ? (
        <QueryLoading label="正在加载帖子" />
      ) : topics.isError ? (
        <QueryError />
      ) : (
        <>
          <TopicList topics={topics.data?.items ?? []} />
          {topics.data ? (
            <nav className="mt-7 flex items-center justify-between text-sm" aria-label="分页">
              <PageLink
                href={categoryUrl(slug, sort, page - 1)}
                disabled={page <= 1}
                label="上一页"
                icon={<ChevronLeft className="size-4" />}
              />
              <span className="text-muted-foreground">
                第 {topics.data.pagination.page} / {Math.max(1, topics.data.pagination.total_pages)}{" "}
                页
              </span>
              <PageLink
                href={categoryUrl(slug, sort, page + 1)}
                disabled={page >= topics.data.pagination.total_pages}
                label="下一页"
                icon={<ChevronRight className="size-4" />}
                iconAfter
              />
            </nav>
          ) : null}
        </>
      )}
    </main>
  );
}

function categoryUrl(slug: string, sort: TopicSort, page: number): Route {
  return `/categories/${encodeURIComponent(slug)}?sort=${sort}&page=${Math.max(1, page)}` as Route;
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
