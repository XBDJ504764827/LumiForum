"use client";

import type { AdminCommentListParams } from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
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
import {
  adminKeys,
  deleteAdminComment,
  listAdminComments,
  restoreAdminComment,
} from "@/lib/api/admin";

export function AdminCommentsView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<AdminCommentListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const [error, setError] = useState<string | null>(null);
  const comments = useQuery({
    queryKey: adminKeys.comments(params),
    queryFn: () => listAdminComments(params),
  });
  const mutation = useMutation({
    mutationFn: async ({ id, restore }: { id: string; restore?: boolean }) => {
      if (restore) return restoreAdminComment(id);
      return deleteAdminComment(id);
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "comments"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (comments.isPending) return <QueryLoading label="正在加载评论" />;
  if (comments.isError) return <QueryError message="无法加载评论列表" />;

  return (
    <div>
      <AdminPageHeader title="评论管理" description="处理违规评论并支持恢复。" />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索评论内容"
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
          <option value="published">已发布</option>
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
      <AdminTable headers={["内容", "帖子", "作者", "状态", "时间", "操作"]}>
        {comments.data.items.map((comment) => (
          <tr key={comment.id}>
            <td className="max-w-sm px-3 py-3">
              <p className="line-clamp-2">{comment.content}</p>
            </td>
            <td className="px-3 py-3">
              <div className="font-medium">{comment.topic_title}</div>
              <div className="text-muted-foreground">/{comment.topic_slug}</div>
            </td>
            <td className="px-3 py-3">@{comment.author_username}</td>
            <td className="px-3 py-3">{comment.status}</td>
            <td className="px-3 py-3">{formatDateTime(comment.created_at)}</td>
            <td className="px-3 py-3">
              {comment.status === "published" ? (
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    if (window.confirm("确认删除该评论？")) {
                      mutation.mutate({ id: comment.id });
                    }
                  }}
                >
                  删除
                </Button>
              ) : (
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => mutation.mutate({ id: comment.id, restore: true })}
                >
                  恢复
                </Button>
              )}
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={comments.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
