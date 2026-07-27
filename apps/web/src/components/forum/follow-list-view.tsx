"use client";

import type { UserPublicSummary } from "@lumiforum/types";
import { Avatar, AvatarFallback, AvatarImage, Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Users } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import {
  followUser,
  forumKeys,
  listFollowers,
  listFollowing,
  unfollowUser,
} from "@/lib/api/forum";

type Mode = "followers" | "following";

export function FollowListView({ userId, mode }: { userId: string; mode: Mode }) {
  const { status, user } = useAuth();
  const queryClient = useQueryClient();
  const params = { page: 1, page_size: 30 };
  const queryKey =
    mode === "followers" ? forumKeys.followers(userId, params) : forumKeys.following(userId, params);
  const list = useQuery({
    queryKey,
    queryFn: () =>
      mode === "followers" ? listFollowers(userId, params) : listFollowing(userId, params),
  });

  const toggle = useMutation({
    mutationFn: async (target: UserPublicSummary) => {
      return target.is_following ? unfollowUser(target.id) : followUser(target.id);
    },
    onMutate: async (target) => {
      await queryClient.cancelQueries({ queryKey });
      const previous = queryClient.getQueryData(queryKey);
      queryClient.setQueryData(queryKey, (current: typeof list.data) => {
        if (!current) return current;
        return {
          ...current,
          items: current.items.map((item) =>
            item.id === target.id ? { ...item, is_following: !item.is_following } : item,
          ),
        };
      });
      return { previous };
    },
    onError: (_error, _target, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKey, context.previous);
      }
    },
    onSuccess: (result, target) => {
      queryClient.setQueryData(queryKey, (current: typeof list.data) => {
        if (!current) return current;
        return {
          ...current,
          items: current.items.map((item) =>
            item.id === target.id
              ? {
                  ...item,
                  is_following: result.following,
                  followers_count: result.followers_count,
                }
              : item,
          ),
        };
      });
    },
  });

  if (list.isPending) return <QueryLoading label="正在加载用户列表" />;
  if (list.isError) return <QueryError message="用户列表加载失败" />;

  const title = mode === "followers" ? "粉丝" : "关注";

  return (
    <main className="mx-auto max-w-3xl px-5 py-9 sm:px-8">
      <div className="mb-8 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <span>{title}</span>
        </div>
        <h1 className="mt-3 flex items-center gap-2 text-3xl font-semibold">
          <Users className="size-7" aria-hidden="true" />
          {title}
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">共 {list.data.pagination.total} 人</p>
      </div>

      {list.data.items.length === 0 ? (
        <p className="border-y border-border py-12 text-center text-sm text-muted-foreground">
          暂无{title}
        </p>
      ) : (
        <ul className="divide-y divide-border border-y border-border">
          {list.data.items.map((item) => {
            const isSelf = user?.id === item.id;
            return (
              <li key={item.id} className="flex items-center justify-between gap-4 py-4">
                <div className="flex min-w-0 items-center gap-3">
                  <Avatar className="size-10 border border-border">
                    {item.avatar ? <AvatarImage src={item.avatar} alt="" /> : null}
                    <AvatarFallback>
                      {(item.nickname || item.username).slice(0, 2).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <div className="min-w-0">
                    <p className="truncate font-medium">{item.nickname || item.username}</p>
                    <p className="text-xs text-muted-foreground">
                      @{item.username} · {item.followers_count} 粉丝
                    </p>
                  </div>
                </div>
                {status === "authenticated" && !isSelf ? (
                  <Button
                    type="button"
                    size="sm"
                    variant={item.is_following ? "outline" : "default"}
                    disabled={toggle.isPending}
                    onClick={() => toggle.mutate(item)}
                  >
                    {item.is_following ? "已关注" : "关注"}
                  </Button>
                ) : null}
              </li>
            );
          })}
        </ul>
      )}
    </main>
  );
}
