"use client";

import type { AdminLogListParams } from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import {
  AdminPageHeader,
  AdminPagination,
  AdminTable,
  AdminToolbar,
  formatDateTime,
} from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { adminKeys, listAdminLogs } from "@/lib/api/admin";

export function AdminLogsView() {
  const [params, setParams] = useState<AdminLogListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const logs = useQuery({
    queryKey: adminKeys.logs(params),
    queryFn: () => listAdminLogs(params),
  });

  if (logs.isPending) return <QueryLoading label="正在加载日志" />;
  if (logs.isError) return <QueryError message="无法加载操作日志" />;

  return (
    <div>
      <AdminPageHeader title="操作日志" description="审计管理员的关键操作。" />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索摘要 / 管理员"
          className="max-w-xs"
        />
        <Select
          value={params.target_type ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              target_type: event.target.value || undefined,
            }))
          }
        >
          <option value="">全部目标</option>
          <option value="user">用户</option>
          <option value="topic">帖子</option>
          <option value="comment">评论</option>
          <option value="category">分类</option>
          <option value="file">文件</option>
          <option value="role">角色</option>
          <option value="system">系统</option>
        </Select>
        <Button
          type="button"
          onClick={() =>
            setParams((current) => ({ ...current, page: 1, q: q.trim() || undefined }))
          }
        >
          搜索
        </Button>
      </AdminToolbar>
      <AdminTable headers={["时间", "管理员", "操作", "目标", "摘要", "来源"]}>
        {logs.data.items.map((log) => (
          <tr key={log.id}>
            <td className="px-3 py-3 whitespace-nowrap">{formatDateTime(log.created_at)}</td>
            <td className="px-3 py-3">@{log.admin_username}</td>
            <td className="px-3 py-3">{log.action}</td>
            <td className="px-3 py-3">
              {log.target_type}
              {log.target_id ? ` · ${log.target_id.slice(0, 8)}` : ""}
            </td>
            <td className="max-w-md px-3 py-3">{log.summary}</td>
            <td className="px-3 py-3 text-muted-foreground">
              <div>{log.ip_address || "-"}</div>
              <div className="line-clamp-1 max-w-xs">{log.user_agent || "-"}</div>
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={logs.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
