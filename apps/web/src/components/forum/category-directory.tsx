"use client";

import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Layers3 } from "lucide-react";
import Link from "next/link";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { forumKeys, listCategories } from "@/lib/api/forum";

export function CategoryDirectory() {
  const categories = useQuery({ queryKey: forumKeys.categories, queryFn: listCategories });

  return (
    <main className="mx-auto max-w-5xl px-5 py-10 sm:px-8">
      <div className="mb-8 border-b border-border pb-7">
        <div className="flex items-center gap-2 text-sm font-medium text-primary">
          <Layers3 className="size-4" aria-hidden="true" />
          论坛导航
        </div>
        <h1 className="mt-2 text-3xl font-semibold">全部板块</h1>
      </div>

      {categories.isPending ? (
        <QueryLoading label="正在加载板块" />
      ) : categories.isError ? (
        <QueryError />
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {(categories.data ?? []).map((category) => (
            <Link
              key={category.id}
              href={`/categories/${category.slug}`}
              className="group grid gap-3 py-6 sm:grid-cols-[minmax(0,1fr)_100px_24px] sm:items-center"
            >
              <div>
                <h2 className="text-lg font-semibold group-hover:text-primary">{category.name}</h2>
                <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">
                  {category.description || "暂无板块说明"}
                </p>
              </div>
              <div className="text-sm sm:text-right">
                <span className="font-semibold">{category.topic_count}</span>
                <span className="ml-1 text-muted-foreground">帖子</span>
              </div>
              <ArrowRight className="hidden size-4 text-muted-foreground group-hover:text-primary sm:block" />
            </Link>
          ))}
          {categories.data?.length === 0 ? (
            <p className="py-14 text-center text-sm text-muted-foreground">暂无可见板块</p>
          ) : null}
        </div>
      )}
    </main>
  );
}
