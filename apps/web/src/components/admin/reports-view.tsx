"use client";

import type { ReportListParams, ReportStatus } from "@lumiforum/types";
import { Button, Select } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  AdminPageHeader,
  AdminPagination,
  AdminTable,
  AdminToolbar,
  formatDateTime,
} from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import { adminKeys, listAdminReports, resolveAdminReport } from "@/lib/api/admin";

export function AdminReportsView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<ReportListParams>({ page: 1, page_size: 20 });
  const [error, setError] = useState<string | null>(null);
  const reports = useQuery({
    queryKey: adminKeys.reports(params),
    queryFn: () => listAdminReports(params),
  });
  const mutation = useMutation({
    mutationFn: ({ id, status }: { id: string; status: ReportStatus }) =>
      resolveAdminReport(id, { status }),
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "reports"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (reports.isPending) return <QueryLoading label="正在加载举报" />;
  if (reports.isError) return <QueryError message="无法加载举报列表" />;

  return (
    <div>
      <AdminPageHeader title="举报管理" description="审核用户举报并记录处理结果。" />
      <AdminToolbar>
        <Select
          value={params.status ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              status: (event.target.value || undefined) as ReportStatus | undefined,
            }))
          }
        >
          <option value="">全部状态</option>
          <option value="open">待处理</option>
          <option value="reviewing">处理中</option>
          <option value="resolved">已解决</option>
          <option value="rejected">已驳回</option>
        </Select>
      </AdminToolbar>
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      <AdminTable headers={["目标", "原因", "举报人", "状态", "时间", "操作"]}>
        {reports.data.items.map((report) => (
          <tr key={report.id}>
            <td className="px-3 py-3">
              <div className="font-medium">
                {report.target_type} · {report.target_id.slice(0, 8)}
              </div>
              {report.details ? (
                <div className="text-muted-foreground">{report.details}</div>
              ) : null}
            </td>
            <td className="px-3 py-3">{report.reason}</td>
            <td className="px-3 py-3">@{report.reporter_username}</td>
            <td className="px-3 py-3">{report.status}</td>
            <td className="px-3 py-3">{formatDateTime(report.created_at)}</td>
            <td className="px-3 py-3">
              {report.status === "open" || report.status === "reviewing" ? (
                <div className="flex flex-wrap gap-2">
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => mutation.mutate({ id: report.id, status: "reviewing" })}
                  >
                    受理
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    onClick={() => {
                      if (window.confirm("确认标记为已解决？")) {
                        mutation.mutate({ id: report.id, status: "resolved" });
                      }
                    }}
                  >
                    解决
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      if (window.confirm("确认驳回该举报？")) {
                        mutation.mutate({ id: report.id, status: "rejected" });
                      }
                    }}
                  >
                    驳回
                  </Button>
                </div>
              ) : (
                <span className="text-muted-foreground">
                  {report.handler_username ? `@${report.handler_username}` : "-"}
                </span>
              )}
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={reports.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
