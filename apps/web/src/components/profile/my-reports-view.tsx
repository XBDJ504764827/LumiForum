"use client";

import type { ReportItemV2 } from "@lumiforum/types";
import { useQuery } from "@tanstack/react-query";
import Link from "next/link";

import { AdminPagination, formatDateTime } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { listMyReports, moderationKeys } from "@/lib/api/moderation";
import { useState } from "react";

export function MyReportsView() {
  const [params, setParams] = useState({ page: 1, page_size: 20 });
  const reports = useQuery({
    queryKey: moderationKeys.myReports(params),
    queryFn: () => listMyReports(params),
  });

  if (reports.isPending) return <QueryLoading label="正在加载举报记录" />;
  if (reports.isError) return <QueryError message="举报记录加载失败" />;

  return (
    <main className="mx-auto max-w-4xl px-5 py-9 sm:px-8">
      <div className="mb-7 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <Link href="/profile" className="hover:text-foreground">
            个人中心
          </Link>
          <span>/</span>
          <span>我的举报</span>
        </div>
        <h1 className="mt-3 text-3xl font-semibold">我的举报</h1>
        <p className="mt-2 text-sm text-muted-foreground">你提交的举报与处理进度</p>
      </div>

      {reports.data.items.length === 0 ? (
        <p className="border-y border-border py-14 text-center text-sm text-muted-foreground">
          还没有提交过举报
        </p>
      ) : (
        <ul className="divide-y divide-border border-y border-border">
          {reports.data.items.map((report: ReportItemV2) => (
            <li key={report.id} className="py-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-sm">
                  <span className="font-medium">{report.reason_code ?? report.reason}</span>
                  <span className="text-muted-foreground">
                    {targetLabel(report.target_type)} · {report.target_id.slice(0, 8)}
                  </span>
                </div>
                <span
                  className={`rounded-sm px-2 py-0.5 text-xs ${
                    report.status === "resolved"
                      ? "bg-emerald-500/10 text-emerald-600"
                      : report.status === "rejected"
                        ? "bg-muted text-muted-foreground"
                        : "bg-amber-500/10 text-amber-600"
                  }`}
                >
                  {statusLabel(report.status)}
                </span>
              </div>
              {report.details ? (
                <p className="mt-1 text-sm text-muted-foreground">{report.details}</p>
              ) : null}
              <div className="mt-1 flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                <span>{formatDateTime(report.created_at)}</span>
                {report.handler_username ? <span>处理人 @{report.handler_username}</span> : null}
                {report.resolution_note ? <span>{report.resolution_note}</span> : null}
              </div>
            </li>
          ))}
        </ul>
      )}

      <div className="mt-6">
        <AdminPagination
          page={reports.data.pagination.page}
          totalPages={reports.data.pagination.total_pages}
          onPageChange={(page) => setParams((current) => ({ ...current, page }))}
        />
      </div>
    </main>
  );
}

function targetLabel(target: string): string {
  return { topic: "帖子", comment: "评论", user: "用户", file: "文件" }[target] ?? target;
}

function statusLabel(status: string): string {
  return (
    {
      open: "待处理",
      reviewing: "处理中",
      resolved: "已解决",
      rejected: "已驳回",
      duplicate: "重复举报",
      cancelled: "已撤回",
    }[status] ?? status
  );
}
