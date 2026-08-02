"use client";

import type { Route } from "next";
import type { TopicDetail } from "@lumiforum/types";
import { Alert, Avatar, AvatarFallback, AvatarImage, Badge, Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Bookmark,
  CalendarDays,
  Eye,
  Heart,
  MessageSquare,
  Pencil,
  Pin,
  Sparkles,
  Trash2,
  UserPlus,
  UserMinus,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { MarkdownContent } from "@/components/forum/markdown-content";
import { PollCard } from "@/components/forum/poll-card";
import { ReportButton } from "@/components/forum/report-button";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import { CommentSection } from "@/components/forum/comment-section";
import { useRealtime } from "@/components/realtime/realtime-provider";
import {
  deleteTopic,
  favoriteTopic,
  followUser,
  forumKeys,
  getTopic,
  likeTopic,
  unfavoriteTopic,
  unfollowUser,
  unlikeTopic,
} from "@/lib/api/forum";
import { getTopicPoll, pollKeys } from "@/lib/api/polls";
import { useEffect } from "react";

const elevatedRoles = new Set(["moderator", "administrator", "super_administrator"]);

export function TopicView({ slug }: { slug: string }) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const { status, user } = useAuth();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const topic = useQuery({
    queryKey: forumKeys.topic(slug),
    queryFn: () => getTopic(slug),
    staleTime: 5 * 60_000,
    retry: false,
  });
  const deletion = useMutation({
    mutationFn: (topicId: string) => deleteTopic(topicId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
      await queryClient.invalidateQueries({ queryKey: forumKeys.categories });
      if (topic.data) {
        router.replace(`/categories/${topic.data.category.slug}` as Route);
      }
    },
  });

  if (topic.isPending) return <QueryLoading label="正在加载帖子" />;
  if (topic.isError || !topic.data) return <QueryError message="帖子不存在或已被删除" />;

  const data = topic.data;
  const canEdit =
    status === "authenticated" &&
    Boolean(user && (user.id === data.author.id || elevatedRoles.has(user.role.code)));
  const canReact = status === "authenticated";
  const canFollow = status === "authenticated" && Boolean(user && user.id !== data.author.id);

  return (
    <main className="mx-auto max-w-5xl px-5 py-9 sm:px-8">
      <div className="mb-6 flex flex-wrap items-center gap-2 text-sm">
        <Link href="/" className="text-muted-foreground hover:text-foreground">
          首页
        </Link>
        <span className="text-muted-foreground">/</span>
        <Link href={`/categories/${data.category.slug}`} className="text-primary hover:underline">
          {data.category.name}
        </Link>
      </div>

      <article>
        <header className="border-b border-border pb-7">
          <div className="mb-3 flex flex-wrap gap-2">
            {data.is_pinned ? (
              <Badge className="gap-1 bg-foreground text-white">
                <Pin className="size-3" />
                置顶
              </Badge>
            ) : null}
            {data.is_featured ? (
              <Badge className="gap-1 bg-accent/15 text-foreground">
                <Sparkles className="size-3" />
                精华
              </Badge>
            ) : null}
          </div>
          <h1 className="max-w-4xl text-3xl font-semibold leading-tight sm:text-4xl">
            {data.title}
          </h1>

          <div className="mt-6 flex flex-col justify-between gap-5 sm:flex-row sm:items-center">
            <div className="flex items-center gap-3">
              <Avatar className="size-10 border border-border">
                {data.author.avatar ? <AvatarImage src={data.author.avatar} alt="" /> : null}
                <AvatarFallback>
                  {authorInitials(data.author.nickname || data.author.username)}
                </AvatarFallback>
              </Avatar>
              <div className="text-sm">
                <p className="font-medium">{data.author.nickname || data.author.username}</p>
                <p className="text-xs text-muted-foreground">{data.author.role.name}</p>
              </div>
              <Link
                href={`/users/${data.author.id}/topics` as Route}
                className="text-xs text-muted-foreground underline-offset-2 hover:text-primary hover:underline"
              >
                TA 的帖子
              </Link>
              {canFollow ? (
                <FollowAuthorButton
                  authorId={data.author.id}
                  following={data.following_author}
                  slug={slug}
                />
              ) : null}
              <ReportButton targetType="user" targetId={data.author.id} />
            </div>
            <div className="flex flex-wrap gap-4 text-xs text-muted-foreground">
              <Meta icon={CalendarDays} value={formatDate(data.created_at)} />
              <Meta icon={Eye} value={`${data.stats.views} 浏览`} />
              <Meta icon={MessageSquare} value={`${data.stats.replies} 回复`} />
              <Meta icon={Heart} value={`${data.stats.likes} 喜欢`} />
            </div>
          </div>
        </header>

        <MarkdownContent content={data.content} className="py-4" />

        <TopicPollSection topicId={data.id} hasPoll={data.has_poll} />

        <div className="mt-6 flex flex-wrap items-center gap-2 border-t border-border pt-6">
          <ReportButton
            targetType="topic"
            targetId={data.id}
            className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-destructive"
          />
          {canReact ? (
            <>
              <TopicLikeButton topic={data} slug={slug} />
              <TopicFavoriteButton topic={data} slug={slug} />
            </>
          ) : status === "unauthenticated" ? (
            <p className="text-sm text-muted-foreground">
              <Link href="/login" className="font-medium text-primary hover:underline">
                登录
              </Link>
              后可点赞与收藏
            </p>
          ) : null}
        </div>

        {canEdit ? (
          <footer className="mt-8 border-t border-border pt-6">
            <div className="flex flex-wrap items-center gap-2">
              <Link
                href={`/topics/${data.slug}/edit`}
                className="inline-flex h-9 items-center gap-2 rounded-md border border-border px-3 text-sm font-medium hover:bg-muted"
              >
                <Pencil className="size-4" aria-hidden="true" />
                编辑
              </Link>
              <Button
                variant="ghost"
                size="sm"
                className="gap-2 text-destructive hover:bg-destructive/5"
                onClick={() => setConfirmDelete(true)}
              >
                <Trash2 className="size-4" aria-hidden="true" />
                删除
              </Button>
            </div>
            {confirmDelete ? (
              <div className="mt-4 flex flex-col justify-between gap-4 border border-destructive/30 bg-destructive/5 p-4 sm:flex-row sm:items-center">
                <p className="text-sm text-destructive">帖子将被软删除，确认继续？</p>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                    取消
                  </Button>
                  <Button
                    size="sm"
                    className="gap-2 bg-destructive text-white hover:bg-destructive/90"
                    disabled={deletion.isPending}
                    onClick={() => deletion.mutate(data.id)}
                  >
                    {deletion.isPending ? <LoadingIndicator /> : <Trash2 className="size-4" />}
                    确认删除
                  </Button>
                </div>
              </div>
            ) : null}
            {deletion.isError ? (
              <Alert className="mt-4">删除失败，请确认权限或稍后重试。</Alert>
            ) : null}
          </footer>
        ) : null}
      </article>

      <CommentSection topicId={data.id} />
    </main>
  );
}

function TopicPollSection({ topicId, hasPoll }: { topicId: string; hasPoll: boolean }) {
  const { client } = useRealtime();
  const queryClient = useQueryClient();
  const poll = useQuery({
    queryKey: pollKeys.topicPoll(topicId),
    queryFn: () => getTopicPoll(topicId),
    enabled: hasPoll,
    staleTime: 30_000,
    retry: false,
  });

  // Realtime: if the poll was deleted while viewing, drop the card.
  useEffect(() => {
    if (!client || !hasPoll) return;
    const off = client.onMessage((message) => {
      if (message.type === "poll.updated") {
        const data = message.data as { poll_id?: string; event?: string };
        if (data.event === "deleted") {
          void queryClient.invalidateQueries({ queryKey: pollKeys.topicPoll(topicId) });
        }
      }
    });
    return off;
  }, [client, hasPoll, topicId, queryClient]);

  if (!hasPoll) return null;
  if (poll.isPending) return <QueryLoading label="正在加载投票" />;
  if (poll.isError || !poll.data) return null;
  return <PollCard poll={poll.data} />;
}

function TopicLikeButton({ topic, slug }: { topic: TopicDetail; slug: string }) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: () => (topic.liked_by_me ? unlikeTopic(topic.id) : likeTopic(topic.id)),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: forumKeys.topic(slug) });
      const previous = queryClient.getQueryData<TopicDetail>(forumKeys.topic(slug));
      if (previous) {
        const liked = !previous.liked_by_me;
        queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), {
          ...previous,
          liked_by_me: liked,
          stats: {
            ...previous.stats,
            likes: Math.max(0, previous.stats.likes + (liked ? 1 : -1)),
          },
        });
      }
      return { previous };
    },
    onError: (_error, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(forumKeys.topic(slug), context.previous);
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), (current) =>
        current
          ? {
              ...current,
              liked_by_me: result.liked,
              stats: { ...current.stats, likes: result.like_count },
            }
          : current,
      );
    },
  });

  return (
    <Button
      type="button"
      variant={topic.liked_by_me ? "default" : "outline"}
      size="sm"
      className="gap-2"
      disabled={mutation.isPending}
      onClick={() => mutation.mutate()}
    >
      <Heart className={`size-4 ${topic.liked_by_me ? "fill-current" : ""}`} aria-hidden="true" />
      {topic.liked_by_me ? "已点赞" : "点赞"}
      <span className="text-xs opacity-80">{topic.stats.likes}</span>
    </Button>
  );
}

function TopicFavoriteButton({ topic, slug }: { topic: TopicDetail; slug: string }) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: () => (topic.favorited_by_me ? unfavoriteTopic(topic.id) : favoriteTopic(topic.id)),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: forumKeys.topic(slug) });
      const previous = queryClient.getQueryData<TopicDetail>(forumKeys.topic(slug));
      if (previous) {
        queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), {
          ...previous,
          favorited_by_me: !previous.favorited_by_me,
        });
      }
      return { previous };
    },
    onError: (_error, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(forumKeys.topic(slug), context.previous);
      }
    },
    onSuccess: async (result) => {
      queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), (current) =>
        current ? { ...current, favorited_by_me: result.favorited } : current,
      );
      await queryClient.invalidateQueries({ queryKey: ["forum", "favorites"] });
    },
  });

  return (
    <Button
      type="button"
      variant={topic.favorited_by_me ? "default" : "outline"}
      size="sm"
      className="gap-2"
      disabled={mutation.isPending}
      onClick={() => mutation.mutate()}
    >
      <Bookmark
        className={`size-4 ${topic.favorited_by_me ? "fill-current" : ""}`}
        aria-hidden="true"
      />
      {topic.favorited_by_me ? "已收藏" : "收藏"}
    </Button>
  );
}

function FollowAuthorButton({
  authorId,
  following,
  slug,
}: {
  authorId: string;
  following: boolean;
  slug: string;
}) {
  const queryClient = useQueryClient();
  const mutation = useMutation({
    mutationFn: () => (following ? unfollowUser(authorId) : followUser(authorId)),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: forumKeys.topic(slug) });
      const previous = queryClient.getQueryData<TopicDetail>(forumKeys.topic(slug));
      if (previous) {
        queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), {
          ...previous,
          following_author: !previous.following_author,
        });
      }
      return { previous };
    },
    onError: (_error, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(forumKeys.topic(slug), context.previous);
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData<TopicDetail>(forumKeys.topic(slug), (current) =>
        current ? { ...current, following_author: result.following } : current,
      );
    },
  });

  return (
    <Button
      type="button"
      variant={following ? "outline" : "default"}
      size="sm"
      className="gap-1.5"
      disabled={mutation.isPending}
      onClick={() => mutation.mutate()}
    >
      {following ? (
        <UserMinus className="size-3.5" aria-hidden="true" />
      ) : (
        <UserPlus className="size-3.5" aria-hidden="true" />
      )}
      {following ? "已关注" : "关注"}
    </Button>
  );
}

function Meta({ icon: Icon, value }: { icon: typeof Eye; value: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <Icon className="size-4" aria-hidden="true" />
      {value}
    </span>
  );
}

function authorInitials(value: string): string {
  return value.slice(0, 2).toUpperCase();
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(value),
  );
}
