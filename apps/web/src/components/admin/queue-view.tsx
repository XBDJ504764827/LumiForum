"use client";

import { useQuery } from "@tanstack/react-query";
import { Flag, FolderOpen, Inbox, MessagesSquare, ShieldAlert, Upload } from "lucide-react";
import Link from "next/link";

import { AdminPageHeader, formatDateTime } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { adminKeys, getAdminQueue } from "@/lib/api/admin";

export function AdminQueueView() {
  const queue = useQuery({
    queryKey: adminKeys.queue,
    queryFn: getAdminQueue,
  });

  if (queue.isPending) return <QueryLoading label="正在加载审核队列" />;
  if (queue.isError || !queue.data) return <QueryError message="审核队列加载失败" />;

  const data = queue.data;
  const cards = [
    {
      label: "待处理举报",
      value: data.pending_reports,
      icon: Flag,
      href: "/admin/reports?status=open",
    },
    {
      label: "处理中举报",
      value: data.reviewing_reports,
      icon: Inbox,
      href: "/admin/reports?status=reviewing",
    },
    { label: "未结案件", value: data.open_cases, icon: ShieldAlert, href: "/admin/moderation" },
    {
      label: "隐藏帖子",
      value: data.hidden_topics,
      icon: FolderOpen,
      href: "/admin/topics?status=hidden",
    },
    {
      label: "隐藏评论",
      value: data.hidden_comments,
      icon: MessagesSquare,
      href: "/admin/comments?status=hidden",
    },
    { label: "待审文件", value: data.pending_uploads, icon: Upload, href: "/admin/files" },
  ];

  return (
    <div>
      <AdminPageHeader title="审核队列" description="待处理内容与案件总览。" />

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {cards.map((card) => (
          <Link
            key={card.label}
            href={card.href}
            className="flex items-center justify-between border border-border bg-white px-4 py-4 hover:border-primary/40"
          >
            <span className="flex items-center gap-2 text-sm text-muted-foreground">
              <card.icon className="size-4" aria-hidden="true" />
              {card.label}
            </span>
            <span className="text-xl font-semibold tabular-nums">{card.value}</span>
          </Link>
        ))}
      </div>

      <div className="mt-6 grid gap-6 xl:grid-cols-2">
        <section className="border border-border bg-white p-5">
          <h2 className="mb-3 font-semibold">最新举报</h2>
          {data.latest_reports.length === 0 ? (
            <p className="text-sm text-muted-foreground">队列为空</p>
          ) : (
            <ul className="divide-y divide-border">
              {data.latest_reports.map((report) => (
                <li
                  key={report.id}
                  className="flex items-center justify-between gap-3 py-2.5 text-sm"
                >
                  <span className="truncate">
                    <span className="font-medium">{report.reason}</span>
                    <span className="ml-2 text-xs text-muted-foreground">
                      {report.target_type} · 举报人 @{report.reporter_username}
                    </span>
                  </span>
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {formatDateTime(report.created_at)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
        <section className="border border-border bg-white p-5">
          <h2 className="mb-3 font-semibold">未结案件</h2>
          {data.latest_cases.length === 0 ? (
            <p className="text-sm text-muted-foreground">队列为空</p>
          ) : (
            <ul className="divide-y divide-border">
              {data.latest_cases.map((item) => (
                <li
                  key={item.id}
                  className="flex items-center justify-between gap-3 py-2.5 text-sm"
                >
                  <span className="truncate">
                    <span className="font-medium">{item.target_type}</span>
                    <span className="ml-2 text-xs text-muted-foreground">
                      {item.source} · {item.priority}
                    </span>
                  </span>
                  <span className="shrink-0 text-xs text-muted-foreground">
                    {formatDateTime(item.opened_at)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
