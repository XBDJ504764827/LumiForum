"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import type { CommentNode, Paginated, User } from "@lumiforum/types";
import { Alert, Avatar, AvatarFallback, AvatarImage, Button, Textarea } from "@lumiforum/ui";
import {
  useInfiniteQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import type { InfiniteData } from "@tanstack/react-query";
import { ChevronDown, Heart, MessageSquare, Pencil, Reply, Trash2 } from "lucide-react";
import Link from "next/link";
import { useMemo, useState } from "react";
import { useForm, useWatch } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { MarkdownContent } from "@/components/forum/markdown-content";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage } from "@/lib/api/errors";
import {
  createComment,
  deleteComment,
  forumKeys,
  likeComment,
  listComments,
  replyToComment,
  unlikeComment,
  updateComment,
} from "@/lib/api/forum";
import { commentEditorSchema, type CommentEditorValues } from "@/lib/forum/comment-schemas";

const elevatedRoles = new Set(["moderator", "administrator", "super_administrator"]);
const PAGE_SIZE = 20;

export function CommentSection({ topicId }: { topicId: string }) {
  const { status, user } = useAuth();
  const queryClient = useQueryClient();
  const comments = useInfiniteQuery({
    queryKey: forumKeys.comments(topicId, { page_size: PAGE_SIZE }),
    queryFn: ({ pageParam }) => listComments(topicId, { page: pageParam, page_size: PAGE_SIZE }),
    initialPageParam: 1,
    getNextPageParam: (last) =>
      last.pagination.page < last.pagination.total_pages ? last.pagination.page + 1 : undefined,
  });

  const invalidate = async () => {
    await queryClient.invalidateQueries({
      queryKey: ["forum", "comments", topicId],
    });
    await queryClient.invalidateQueries({ queryKey: ["forum", "topic"] });
    await queryClient.invalidateQueries({ queryKey: ["forum", "topics"] });
  };

  const items = useMemo(
    () => comments.data?.pages.flatMap((page) => page.items) ?? [],
    [comments.data],
  );
  const total = comments.data?.pages[0]?.pagination.total ?? 0;

  return (
    <section className="mt-12 border-t border-border pt-8" aria-labelledby="comments-title">
      <div className="mb-6 flex items-center justify-between gap-4">
        <h2 id="comments-title" className="flex items-center gap-2 text-xl font-semibold">
          <MessageSquare className="size-5" aria-hidden="true" />
          评论 {total > 0 ? `(${total})` : ""}
        </h2>
      </div>

      {status === "authenticated" ? (
        <CommentComposer
          title="发表评论"
          submitLabel="发布评论"
          onSubmit={async (content) => {
            await createComment(topicId, { content });
            await invalidate();
          }}
        />
      ) : status === "unauthenticated" ? (
        <Alert className="mb-6">
          <Link href="/login" className="font-medium text-primary hover:underline">
            登录
          </Link>
          后参与讨论
        </Alert>
      ) : null}

      {comments.isPending ? (
        <QueryLoading label="正在加载评论" />
      ) : comments.isError ? (
        <QueryError message="评论加载失败" />
      ) : items.length === 0 ? (
        <p className="border-y border-border py-10 text-center text-sm text-muted-foreground">
          还没有评论，来抢沙发吧
        </p>
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {items.map((comment) => (
            <CommentItem
              key={comment.id}
              comment={comment}
              topicId={topicId}
              user={user}
              onChanged={invalidate}
            />
          ))}
        </div>
      )}

      {comments.hasNextPage ? (
        <div className="mt-6 flex justify-center">
          <Button
            variant="outline"
            className="gap-2"
            disabled={comments.isFetchingNextPage}
            onClick={() => comments.fetchNextPage()}
          >
            {comments.isFetchingNextPage ? (
              <LoadingIndicator />
            ) : (
              <ChevronDown className="size-4" aria-hidden="true" />
            )}
            加载更多评论
          </Button>
        </div>
      ) : null}
    </section>
  );
}

function CommentItem({
  comment,
  topicId,
  user,
  onChanged,
  isChild = false,
}: {
  comment: CommentNode;
  topicId: string;
  user: User | null;
  onChanged: () => Promise<void>;
  isChild?: boolean;
}) {
  const [mode, setMode] = useState<"view" | "reply" | "edit">("view");
  const [confirmDelete, setConfirmDelete] = useState(false);
  const canEdit = Boolean(
    user && (user.id === comment.author.id || elevatedRoles.has(user.role.code)),
  );
  const canReply = Boolean(user) && !isChild;
  const canLike = Boolean(user);

  const deletion = useMutation({
    mutationFn: () => deleteComment(comment.id),
    onSuccess: onChanged,
  });

  return (
    <article className={isChild ? "py-4 pl-4 sm:pl-8" : "py-6"}>
      <div className="flex items-start gap-3">
        <Avatar className="size-9 border border-border">
          {comment.author.avatar ? <AvatarImage src={comment.author.avatar} alt="" /> : null}
          <AvatarFallback>
            {(comment.author.nickname || comment.author.username).slice(0, 2).toUpperCase()}
          </AvatarFallback>
        </Avatar>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
            <span className="font-medium">
              {comment.author.nickname || comment.author.username}
            </span>
            <span className="text-xs text-muted-foreground">{comment.author.role.name}</span>
            <span className="text-xs text-muted-foreground">{formatDate(comment.created_at)}</span>
            {comment.edited_at ? (
              <span className="text-xs text-muted-foreground">已编辑</span>
            ) : null}
          </div>

          {mode === "edit" ? (
            <div className="mt-3">
              <CommentComposer
                title="编辑评论"
                submitLabel="保存"
                defaultValue={comment.content}
                onCancel={() => setMode("view")}
                onSubmit={async (content) => {
                  await updateComment(comment.id, { content });
                  setMode("view");
                  await onChanged();
                }}
              />
            </div>
          ) : (
            <MarkdownContent content={comment.content} className="mt-2 text-sm" />
          )}

          <div className="mt-3 flex flex-wrap items-center gap-2">
            {canLike ? (
              <CommentLikeButton comment={comment} topicId={topicId} />
            ) : (
              <span className="inline-flex items-center gap-1.5 px-2 text-xs text-muted-foreground">
                <Heart className="size-3.5" aria-hidden="true" />
                {comment.stats.likes}
              </span>
            )}
            {canReply ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="gap-1.5"
                onClick={() => setMode(mode === "reply" ? "view" : "reply")}
              >
                <Reply className="size-3.5" aria-hidden="true" />
                回复
              </Button>
            ) : null}
            {canEdit ? (
              <>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="gap-1.5"
                  onClick={() => setMode(mode === "edit" ? "view" : "edit")}
                >
                  <Pencil className="size-3.5" aria-hidden="true" />
                  编辑
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  className="gap-1.5 text-destructive hover:bg-destructive/5"
                  onClick={() => setConfirmDelete(true)}
                >
                  <Trash2 className="size-3.5" aria-hidden="true" />
                  删除
                </Button>
              </>
            ) : null}
          </div>

          {confirmDelete ? (
            <div className="mt-3 flex flex-col gap-3 border border-destructive/30 bg-destructive/5 p-3 sm:flex-row sm:items-center sm:justify-between">
              <p className="text-sm text-destructive">确认软删除这条评论？</p>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" onClick={() => setConfirmDelete(false)}>
                  取消
                </Button>
                <Button
                  size="sm"
                  className="bg-destructive text-white hover:bg-destructive/90"
                  disabled={deletion.isPending}
                  onClick={() => deletion.mutate()}
                >
                  {deletion.isPending ? <LoadingIndicator /> : "确认删除"}
                </Button>
              </div>
            </div>
          ) : null}
          {deletion.isError ? <Alert className="mt-3">{errorMessage(deletion.error)}</Alert> : null}

          {mode === "reply" ? (
            <div className="mt-4 border-l border-border pl-4">
              <CommentComposer
                title={`回复 ${comment.author.nickname || comment.author.username}`}
                submitLabel="发布回复"
                onCancel={() => setMode("view")}
                onSubmit={async (content) => {
                  await replyToComment(comment.id, { content });
                  setMode("view");
                  await onChanged();
                }}
              />
            </div>
          ) : null}

          {comment.replies.length > 0 ? (
            <div className="mt-4 divide-y divide-border border-t border-border">
              {comment.replies.map((reply) => (
                <CommentItem
                  key={reply.id}
                  comment={reply}
                  topicId={topicId}
                  user={user}
                  onChanged={onChanged}
                  isChild
                />
              ))}
            </div>
          ) : null}
        </div>
      </div>
    </article>
  );
}

function CommentLikeButton({ comment, topicId }: { comment: CommentNode; topicId: string }) {
  const queryClient = useQueryClient();
  const commentsKey = forumKeys.comments(topicId, { page_size: PAGE_SIZE });
  const mutation = useMutation({
    mutationFn: () =>
      comment.liked_by_me ? unlikeComment(comment.id) : likeComment(comment.id),
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: ["forum", "comments", topicId] });
      const previous =
        queryClient.getQueryData<InfiniteData<Paginated<CommentNode>>>(commentsKey);
      if (previous) {
        queryClient.setQueryData<InfiniteData<Paginated<CommentNode>>>(commentsKey, {
          ...previous,
          pages: previous.pages.map((page) => ({
            ...page,
            items: mapCommentTree(page.items, comment.id, (node) => {
              const liked = !node.liked_by_me;
              return {
                ...node,
                liked_by_me: liked,
                stats: {
                  ...node.stats,
                  likes: Math.max(0, node.stats.likes + (liked ? 1 : -1)),
                },
              };
            }),
          })),
        });
      }
      return { previous };
    },
    onError: (_error, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(commentsKey, context.previous);
      }
    },
    onSuccess: (result) => {
      queryClient.setQueryData<InfiniteData<Paginated<CommentNode>>>(commentsKey, (current) => {
        if (!current) return current;
        return {
          ...current,
          pages: current.pages.map((page) => ({
            ...page,
            items: mapCommentTree(page.items, comment.id, (node) => ({
              ...node,
              liked_by_me: result.liked,
              stats: { ...node.stats, likes: result.like_count },
            })),
          })),
        };
      });
    },
  });

  return (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      className={`gap-1.5 ${comment.liked_by_me ? "text-primary" : ""}`}
      disabled={mutation.isPending}
      onClick={() => mutation.mutate()}
    >
      <Heart
        className={`size-3.5 ${comment.liked_by_me ? "fill-current" : ""}`}
        aria-hidden="true"
      />
      {comment.stats.likes > 0 ? comment.stats.likes : "赞"}
    </Button>
  );
}

function mapCommentTree(
  nodes: CommentNode[],
  targetId: string,
  mapper: (node: CommentNode) => CommentNode,
): CommentNode[] {
  return nodes.map((node) => {
    if (node.id === targetId) {
      return mapper(node);
    }
    if (node.replies.length === 0) {
      return node;
    }
    return {
      ...node,
      replies: mapCommentTree(node.replies, targetId, mapper),
    };
  });
}

function CommentComposer({
  title,
  submitLabel,
  defaultValue = "",
  onSubmit,
  onCancel,
}: {
  title: string;
  submitLabel: string;
  defaultValue?: string;
  onSubmit: (content: string) => Promise<void>;
  onCancel?: () => void;
}) {
  const [preview, setPreview] = useState(false);
  const form = useForm<CommentEditorValues>({
    resolver: zodResolver(commentEditorSchema),
    defaultValues: { content: defaultValue },
  });
  const content = useWatch({ control: form.control, name: "content" }) ?? "";
  const mutation = useMutation({
    mutationFn: (values: CommentEditorValues) => onSubmit(values.content),
    onError: (error) => form.setError("root", { message: errorMessage(error) }),
    onSuccess: () => {
      form.reset({ content: "" });
      setPreview(false);
    },
  });

  return (
    <div className="mb-8 rounded-md border border-border p-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <p className="text-sm font-medium">{title}</p>
        <div className="inline-flex rounded-md border border-border p-0.5">
          <button
            type="button"
            className={`h-7 rounded-sm px-2 text-xs ${preview ? "text-muted-foreground" : "bg-muted"}`}
            onClick={() => setPreview(false)}
          >
            编写
          </button>
          <button
            type="button"
            className={`h-7 rounded-sm px-2 text-xs ${preview ? "bg-muted" : "text-muted-foreground"}`}
            onClick={() => setPreview(true)}
          >
            预览
          </button>
        </div>
      </div>
      <form className="space-y-3" onSubmit={form.handleSubmit((values) => mutation.mutate(values))}>
        {form.formState.errors.root?.message ? (
          <Alert>{form.formState.errors.root.message}</Alert>
        ) : null}
        {preview ? (
          <div className="min-h-28 border border-border bg-white px-3 py-2">
            {content ? (
              <MarkdownContent content={content} className="text-sm" />
            ) : (
              <p className="text-sm text-muted-foreground">暂无可预览内容</p>
            )}
          </div>
        ) : (
          <Textarea
            className="min-h-28 font-mono"
            placeholder="支持 Markdown：列表、引用、代码块、链接"
            aria-invalid={Boolean(form.formState.errors.content)}
            {...form.register("content")}
          />
        )}
        <p className="min-h-5 text-sm text-destructive">{form.formState.errors.content?.message}</p>
        <div className="flex flex-wrap gap-2">
          <Button type="submit" size="sm" className="gap-2" disabled={mutation.isPending}>
            {mutation.isPending ? <LoadingIndicator /> : null}
            {submitLabel}
          </Button>
          {onCancel ? (
            <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
              取消
            </Button>
          ) : null}
        </div>
      </form>
    </div>
  );
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
