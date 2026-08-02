"use client";

import type { AdminPollListParams } from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BarChart3 } from "lucide-react";
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
import { adminKeys, listAdminPolls } from "@/lib/api/admin";
import { closePoll, deletePoll } from "@/lib/api/polls";

export function AdminPollsView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<AdminPollListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const [error, setError] = useState<string | null>(null);
  const polls = useQuery({
    queryKey: adminKeys.polls(params),
    queryFn: () => listAdminPolls(params),
  });
  const mutation = useMutation({
    mutationFn: async ({ id, action }: { id: string; action: "close" | "delete" }) => {
      if (action === "close") return closePoll(id);
      return deletePoll(id);
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "polls"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (polls.isPending) return <QueryLoading label="正在加载投票" />;
  if (polls.isError) return <QueryError message="无法加载投票列表" />;

  return (
    <div>
      <AdminPageHeader title="投票管理" description="查看、关闭或删除违规投票。" />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索投票标题 / 帖子标题"
          className="max-w-xs"
        />
        <Select
          value={params.status ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              status: (event.target.value as AdminPollListParams["status"]) || undefined,
            }))
          }
        >
          <option value="">全部状态</option>
          <option value="active">进行中</option>
          <option value="closed">已结束</option>
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
      <AdminTable headers={["投票", "帖子", "作者", "状态", "统计", "时间", "操作"]}>
        {polls.data.items.map((poll) => (
          <tr key={poll.id}>
            <td className="px-3 py-3">
              <div className="flex items-center gap-2 font-medium">
                <BarChart3 className="size-4 shrink-0 text-primary" aria-hidden="true" />
                {poll.title}
              </div>
              <div className="mt-0.5 text-xs text-muted-foreground">
                {poll.multiple_choice ? `多选 · 最多 ${poll.max_choices} 项` : "单选"}
                {poll.anonymous ? " · 匿名" : ""}
              </div>
            </td>
            <td className="px-3 py-3">
              <a
                href={`/topics/${poll.topic_slug}`}
                target="_blank"
                rel="noreferrer"
                className="text-primary hover:underline"
              >
                {poll.topic_title}
              </a>
            </td>
            <td className="px-3 py-3">@{poll.author_username}</td>
            <td className="px-3 py-3">
              {poll.status === "active" ? (
                <span className="text-emerald-600">进行中</span>
              ) : (
                <span className="text-muted-foreground">已结束</span>
              )}
            </td>
            <td className="px-3 py-3 text-muted-foreground">
              {poll.option_count} 选项 / {poll.participant_count} 人参与
              {poll.expires_at ? (
                <div className="text-xs">截止 {formatDateTime(poll.expires_at)}</div>
              ) : null}
            </td>
            <td className="px-3 py-3">{formatDateTime(poll.created_at)}</td>
            <td className="px-3 py-3">
              <div className="flex flex-wrap gap-2">
                {poll.status === "active" ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => mutation.mutate({ id: poll.id, action: "close" })}
                  >
                    关闭
                  </Button>
                ) : null}
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  className="text-destructive hover:text-destructive"
                  onClick={() => {
                    if (window.confirm(`确定删除投票「${poll.title}」吗？`)) {
                      mutation.mutate({ id: poll.id, action: "delete" });
                    }
                  }}
                >
                  删除
                </Button>
              </div>
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={polls.data.pagination.page}
        totalPages={polls.data.pagination.total_pages}
        onPageChange={(page: number) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
