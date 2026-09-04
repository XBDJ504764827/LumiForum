"use client";

import type { Paginated, TopicSummary, UserPublicSummary } from "@lumiforum/types";
import { Avatar, AvatarFallback, AvatarImage, Badge, Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarDays, Mail, UserMinus, UserPlus } from "lucide-react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { useMemo, useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicList } from "@/components/forum/topic-list";
import { startConversation } from "@/lib/api/dm";
import { followUser, forumKeys, getPublicUser, listTopics, unfollowUser } from "@/lib/api/forum";
import { errorMessage } from "@/lib/api/errors";

export function UserProfileView({
  userId,
  initialUser,
  initialTopics,
}: {
  userId: string;
  initialUser?: UserPublicSummary;
  initialTopics?: Paginated<TopicSummary>;
}) {
  const { status, user } = useAuth();
  const router = useRouter();
  const queryClient = useQueryClient();
  const profile = useQuery({
    queryKey: ["forum", "user", userId],
    queryFn: () => getPublicUser(userId),
    staleTime: 5 * 60_000,
    retry: false,
    initialData: initialUser,
  });
  const topicsParams = useMemo(
    () => ({ author_id: userId, sort: "latest" as const, page: 1, page_size: 20 }),
    [userId],
  );
  const topics = useQuery({
    queryKey: forumKeys.topics(topicsParams),
    queryFn: () => listTopics(topicsParams),
    staleTime: 60_000,
    initialData: initialTopics,
  });

  const toggleFollow = useMutation({
    mutationFn: async (target: UserPublicSummary) =>
      target.is_following ? unfollowUser(target.id) : followUser(target.id),
    onError: () => undefined,
    onSuccess: (result) => {
      queryClient.setQueryData<UserPublicSummary>(["forum", "user", userId], (current) =>
        current
          ? {
              ...current,
              is_following: result.following,
              followers_count: result.followers_count,
            }
          : current,
      );
    },
  });
  const [dmStarting, setDmStarting] = useState(false);

  if (profile.isPending) return <QueryLoading label="正在加载用户" />;
  if (profile.isError || !profile.data) {
    return (
      <QueryError
        message={
          errorMessage(profile.error) === "请先登录"
            ? "登录已过期，请重新登录"
            : "用户不存在或不可见"
        }
      />
    );
  }

  const data = profile.data;
  const canFollow = status === "authenticated" && Boolean(user && user.id !== data.id);

  const startDm = async () => {
    setDmStarting(true);
    try {
      const conversation = await startConversation(data.username);
      router.push(`/messages?c=${conversation.id}` as never);
    } catch {
      // 不可达（对方不存在/被禁用/给自己）时静默返回，不打断浏览。
    } finally {
      setDmStarting(false);
    }
  };

  return (
    <main className="mx-auto max-w-5xl px-5 py-10 sm:px-8">
      <section className="flex flex-col gap-5 border-b border-border pb-8 sm:flex-row sm:items-center">
        <Avatar className="size-20 border border-border">
          {data.avatar ? <AvatarImage src={data.avatar} alt="" /> : null}
          <AvatarFallback>
            {data.nickname?.[0] ?? data.username[0]?.toUpperCase() ?? "?"}
          </AvatarFallback>
        </Avatar>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="text-2xl font-semibold">{data.nickname || data.username}</h1>
            <Badge className="bg-primary/10 text-primary">{data.role.name}</Badge>
          </div>
          <p className="mt-1 text-sm text-muted-foreground">@{data.username}</p>
          <p className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm text-muted-foreground">
            <span>{data.followers_count} 粉丝</span>
            <span>{data.following_count} 关注</span>
            <span className="inline-flex items-center gap-1">
              <CalendarDays className="size-3.5" aria-hidden="true" />
              {new Date(data.created_at).toLocaleDateString("zh-CN")} 加入
            </span>
          </p>
        </div>
        {canFollow ? (
          <div className="flex shrink-0 gap-2">
            <Button
              type="button"
              variant="outline"
              className="gap-2"
              disabled={dmStarting}
              onClick={() => void startDm()}
            >
              <Mail className="size-4" aria-hidden="true" />
              发私信
            </Button>
            <Button
              type="button"
              variant={data.is_following ? "outline" : "default"}
              className="gap-2"
              disabled={toggleFollow.isPending}
              onClick={() => toggleFollow.mutate(data)}
            >
              {data.is_following ? (
                <UserMinus className="size-4" aria-hidden="true" />
              ) : (
                <UserPlus className="size-4" aria-hidden="true" />
              )}
              {data.is_following ? "取消关注" : "关注"}
            </Button>
          </div>
        ) : null}
      </section>

      <section className="mt-8" aria-labelledby="user-topics-title">
        <div className="mb-4 flex items-center justify-between">
          <h2 id="user-topics-title" className="text-lg font-semibold">
            发布的帖子
          </h2>
          <Link href={`/users/${userId}/topics`} className="text-sm text-primary hover:underline">
            查看全部
          </Link>
        </div>
        {topics.isPending ? (
          <QueryLoading label="正在加载帖子" />
        ) : topics.isError ? (
          <QueryError />
        ) : (
          <TopicList topics={topics.data?.items ?? []} />
        )}
      </section>
    </main>
  );
}
