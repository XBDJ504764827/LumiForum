"use client";

import type { AdminFileListParams } from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  AdminPageHeader,
  AdminPagination,
  AdminTable,
  AdminToolbar,
  formatBytes,
  formatDateTime,
} from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import { adminKeys, cleanupAdminFiles, deleteAdminFile, listAdminFiles } from "@/lib/api/admin";

export function AdminFilesView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<AdminFileListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const files = useQuery({
    queryKey: adminKeys.files(params),
    queryFn: () => listAdminFiles(params),
  });
  const mutation = useMutation({
    mutationFn: async (action: { type: "delete"; id: string } | { type: "cleanup" }) => {
      if (action.type === "delete") return deleteAdminFile(action.id);
      return cleanupAdminFiles();
    },
    onSuccess: async (result) => {
      setError(null);
      if (result && typeof result === "object" && "cleaned" in result) {
        setMessage(`已清理 ${result.cleaned} 个孤立文件`);
      } else {
        setMessage(null);
      }
      await queryClient.invalidateQueries({ queryKey: ["admin", "files"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (files.isPending) return <QueryLoading label="正在加载文件" />;
  if (files.isError) return <QueryError message="无法加载文件列表" />;

  return (
    <div>
      <AdminPageHeader
        title="文件管理"
        description="查看上传文件并清理无效对象。"
        actions={
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              if (window.confirm("确认清理超过 24 小时的 pending/failed 文件？")) {
                mutation.mutate({ type: "cleanup" });
              }
            }}
          >
            清理孤立文件
          </Button>
        }
      />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索文件名"
          className="max-w-xs"
        />
        <Select
          value={params.status ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              status: event.target.value || undefined,
            }))
          }
        >
          <option value="">全部状态</option>
          <option value="ready">就绪</option>
          <option value="pending">处理中</option>
          <option value="failed">失败</option>
          <option value="deleted">已删除</option>
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
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      {message ? <p className="mb-3 text-sm text-primary">{message}</p> : null}
      <AdminTable headers={["文件", "用户", "类型", "大小", "状态", "时间", "操作"]}>
        {files.data.items.map((file) => (
          <tr key={file.id}>
            <td className="px-3 py-3">
              <div className="font-medium">{file.original_filename}</div>
              <div className="text-muted-foreground">{file.category}</div>
            </td>
            <td className="px-3 py-3">@{file.username}</td>
            <td className="px-3 py-3">{file.mime_type}</td>
            <td className="px-3 py-3">{formatBytes(file.file_size)}</td>
            <td className="px-3 py-3">{file.status}</td>
            <td className="px-3 py-3">{formatDateTime(file.created_at)}</td>
            <td className="px-3 py-3">
              {file.status !== "deleted" ? (
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    if (window.confirm(`确认删除文件「${file.original_filename}」？`)) {
                      mutation.mutate({ type: "delete", id: file.id });
                    }
                  }}
                >
                  删除
                </Button>
              ) : null}
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={files.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
