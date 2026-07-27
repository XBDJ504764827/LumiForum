"use client";

import { useQuery } from "@tanstack/react-query";
import { Bookmark } from "lucide-react";
import Link from "next/link";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicList } from "@/components/forum/topic-list";
import { forumKeys, listMyFavorites } from "@/lib/api/forum";

export function FavoritesView() {
  const favorites = useQuery({
    queryKey: forumKeys.favorites({ page: 1, page_size: 20 }),
    queryFn: () => listMyFavorites({ page: 1, page_size: 20 }),
  });

  if (favorites.isPending) return <QueryLoading label="正在加载收藏" />;
  if (favorites.isError) return <QueryError message="收藏列表加载失败" />;

  const topics = favorites.data.items.map((item) => item.topic);

  return (
    <main className="mx-auto max-w-5xl px-5 py-9 sm:px-8">
      <div className="mb-8 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <span>我的收藏</span>
        </div>
        <h1 className="mt-3 flex items-center gap-2 text-3xl font-semibold">
          <Bookmark className="size-7" aria-hidden="true" />
          我的收藏
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          共 {favorites.data.pagination.total} 篇收藏帖子
        </p>
      </div>
      <TopicList topics={topics} />
    </main>
  );
}
