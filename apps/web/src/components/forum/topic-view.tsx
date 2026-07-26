"use client";

import type { Route } from "next";
import { Alert, Avatar, AvatarFallback, AvatarImage, Badge, Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  CalendarDays,
  Eye,
  Heart,
  MessageSquare,
  Pencil,
  Pin,
  Sparkles,
  Trash2,
} from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { MarkdownContent } from "@/components/forum/markdown-content";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import { deleteTopic, forumKeys, getTopic } from "@/lib/api/forum";

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

        {canEdit ? (
          <footer className="mt-10 border-t border-border pt-6">
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
    </main>
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
