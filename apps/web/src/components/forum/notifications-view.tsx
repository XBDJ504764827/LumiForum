"use client";

import type { Notification, Paginated } from "@lumiforum/types";
import { Avatar, AvatarFallback, AvatarImage, Button } from "@lumiforum/ui";
import {
  useInfiniteQuery,
  useMutation,
  useQueryClient,
  type InfiniteData,
} from "@tanstack/react-query";
import { Bell, CheckCheck } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useMemo } from "react";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { LoadingIndicator } from "@/components/loading-indicator";
import {
  listNotifications,
  markAllNotificationsRead,
  markNotificationRead,
  notificationHref,
  notificationKeys,
} from "@/lib/api/notifications";

const PAGE_SIZE = 20;

export function NotificationsView() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const list = useInfiniteQuery({
    queryKey: notificationKeys.list({ page_size: PAGE_SIZE }),
    queryFn: ({ pageParam }) => listNotifications({ page: pageParam, page_size: PAGE_SIZE }),
    initialPageParam: 1,
    getNextPageParam: (last) =>
      last.pagination.page < last.pagination.total_pages ? last.pagination.page + 1 : undefined,
  });

  const items = useMemo(() => list.data?.pages.flatMap((page) => page.items) ?? [], [list.data]);
  const total = list.data?.pages[0]?.pagination.total ?? 0;
  const unread = items.filter((item) => !item.is_read).length;

  const markOne = useMutation({
    mutationFn: (id: string) => markNotificationRead(id),
    onMutate: async (id) => {
      await queryClient.cancelQueries({ queryKey: notificationKeys.all });
      const previous = queryClient.getQueryData<InfiniteData<Paginated<Notification>>>(
        notificationKeys.list({ page_size: PAGE_SIZE }),
      );
      queryClient.setQueryData<InfiniteData<Paginated<Notification>>>(
        notificationKeys.list({ page_size: PAGE_SIZE }),
        (current) => {
          if (!current) return current;
          return {
            ...current,
            pages: current.pages.map((page) => ({
              ...page,
              items: page.items.map((item) => (item.id === id ? { ...item, is_read: true } : item)),
            })),
          };
        },
      );
      queryClient.setQueryData(notificationKeys.unread, (current: { count: number } | undefined) =>
        current ? { count: Math.max(0, current.count - 1) } : current,
      );
      return { previous };
    },
    onError: (_error, _id, context) => {
      if (context?.previous) {
        queryClient.setQueryData(notificationKeys.list({ page_size: PAGE_SIZE }), context.previous);
      }
    },
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: notificationKeys.unread });
    },
  });

  const markAll = useMutation({
    mutationFn: markAllNotificationsRead,
    onMutate: async () => {
      await queryClient.cancelQueries({ queryKey: notificationKeys.all });
      const previous = queryClient.getQueryData<InfiniteData<Paginated<Notification>>>(
        notificationKeys.list({ page_size: PAGE_SIZE }),
      );
      queryClient.setQueryData<InfiniteData<Paginated<Notification>>>(
        notificationKeys.list({ page_size: PAGE_SIZE }),
        (current) => {
          if (!current) return current;
          return {
            ...current,
            pages: current.pages.map((page) => ({
              ...page,
              items: page.items.map((item) => ({ ...item, is_read: true })),
            })),
          };
        },
      );
      queryClient.setQueryData(notificationKeys.unread, { count: 0 });
      return { previous };
    },
    onError: (_error, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(notificationKeys.list({ page_size: PAGE_SIZE }), context.previous);
      }
    },
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: notificationKeys.all });
    },
  });

  const openNotification = async (item: Notification) => {
    if (!item.is_read) {
      markOne.mutate(item.id);
    }
    router.push(notificationHref(item));
  };

  if (list.isPending) return <QueryLoading label="正在加载通知" />;
  if (list.isError) return <QueryError message="通知加载失败" />;

  return (
    <main className="mx-auto max-w-3xl px-5 py-9 sm:px-8">
      <div className="mb-8 flex flex-col gap-4 border-b border-border pb-6 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Link href="/" className="hover:text-foreground">
              首页
            </Link>
            <span>/</span>
            <span>通知</span>
          </div>
          <h1 className="mt-3 flex items-center gap-2 text-3xl font-semibold">
            <Bell className="size-7" aria-hidden="true" />
            通知中心
          </h1>
          <p className="mt-2 text-sm text-muted-foreground">
            共 {total} 条 · 未读 {unread}
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-2"
          disabled={markAll.isPending || unread === 0}
          onClick={() => markAll.mutate()}
        >
          {markAll.isPending ? <LoadingIndicator /> : <CheckCheck className="size-4" />}
          全部已读
        </Button>
      </div>

      {items.length === 0 ? (
        <p className="border-y border-border py-14 text-center text-sm text-muted-foreground">
          暂无通知
        </p>
      ) : (
        <ul className="divide-y divide-border border-y border-border">
          {items.map((item) => (
            <li key={item.id}>
              <button
                type="button"
                className={`flex w-full items-start gap-3 px-1 py-4 text-left hover:bg-muted/40 ${
                  item.is_read ? "" : "bg-primary/5"
                }`}
                onClick={() => openNotification(item)}
              >
                <Avatar className="mt-0.5 size-10 border border-border">
                  {item.actor?.avatar ? <AvatarImage src={item.actor.avatar} alt="" /> : null}
                  <AvatarFallback>
                    {(item.actor?.nickname || item.actor?.username || "系统")
                      .slice(0, 2)
                      .toUpperCase()}
                  </AvatarFallback>
                </Avatar>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-medium">{item.title}</p>
                    {!item.is_read ? (
                      <span className="rounded-full bg-primary px-1.5 py-0.5 text-[10px] font-medium text-primary-foreground">
                        未读
                      </span>
                    ) : null}
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{item.content}</p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    {item.actor ? item.actor.nickname || item.actor.username : "系统"} ·{" "}
                    {formatDate(item.created_at)} · {typeLabel(item.type)}
                  </p>
                </div>
              </button>
            </li>
          ))}
        </ul>
      )}

      {list.hasNextPage ? (
        <div className="mt-6 flex justify-center">
          <Button
            variant="outline"
            disabled={list.isFetchingNextPage}
            onClick={() => list.fetchNextPage()}
          >
            {list.isFetchingNextPage ? <LoadingIndicator /> : "加载更多"}
          </Button>
        </div>
      ) : null}
    </main>
  );
}

function typeLabel(type: Notification["type"]): string {
  const map: Record<Notification["type"], string> = {
    post_liked: "点赞",
    comment_liked: "评论点赞",
    comment_created: "新评论",
    comment_replied: "回复",
    topic_favorited: "收藏",
    user_followed: "关注",
    mentioned: "提及",
    system_message: "系统",
    report_submitted: "举报",
    report_processed: "举报处理",
    content_hidden: "内容隐藏",
    content_deleted: "内容删除",
    topic_locked: "帖子锁定",
    user_warned: "警告",
    user_muted: "禁言",
    user_banned: "封禁",
    sanction_expiring: "处罚即将到期",
    sanction_revoked: "处罚撤销",
    appeal_submitted: "申诉",
    appeal_approved: "申诉通过",
    appeal_rejected: "申诉驳回",
    moderation_inbox: "审核",
    poll_voted: "投票",
    poll_ended: "投票结束",
  };
  return map[type] ?? "通知";
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
