"use client";

import { Alert } from "@lumiforum/ui";
import { useQuery } from "@tanstack/react-query";
import { ShieldAlert } from "lucide-react";
import Link from "next/link";

import { useAuth } from "@/components/auth/auth-provider";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { TopicEditor } from "@/components/forum/topic-editor";
import { forumKeys, getTopic } from "@/lib/api/forum";

const elevatedRoles = new Set(["moderator", "administrator", "super_administrator"]);

export function TopicEditView({ slug }: { slug: string }) {
  const { user } = useAuth();
  const topic = useQuery({
    queryKey: forumKeys.topic(slug),
    queryFn: () => getTopic(slug),
    staleTime: 5 * 60_000,
    retry: false,
  });

  if (topic.isPending) return <QueryLoading label="正在加载帖子" />;
  if (topic.isError || !topic.data) return <QueryError message="帖子不存在或已被删除" />;

  const canEdit = Boolean(
    user && (user.id === topic.data.author.id || elevatedRoles.has(user.role.code)),
  );
  if (!canEdit) {
    return (
      <main className="mx-auto max-w-3xl px-5 py-10 sm:px-8">
        <Alert>
          <ShieldAlert className="size-4 shrink-0" aria-hidden="true" />
          你没有编辑此帖子的权限。
        </Alert>
        <Link
          href={`/topics/${topic.data.slug}`}
          className="mt-5 inline-block text-sm text-primary"
        >
          返回帖子
        </Link>
      </main>
    );
  }

  return <TopicEditor mode="edit" topic={topic.data} />;
}
