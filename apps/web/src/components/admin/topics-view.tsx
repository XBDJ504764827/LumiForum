"use client";

import type { AdminTopicListParams } from "@lumiforum/types";
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
import { adminKeys, deleteAdminTopic, listAdminTopics, updateAdminTopic } from "@/lib/api/admin";

export function AdminTopicsView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<AdminTopicListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const [error, setError] = useState<string | null>(null);
  const topics = useQuery({
    queryKey: adminKeys.topics(params),
    queryFn: () => listAdminTopics(params),
  });
  const mutation = useMutation({
    mutationFn: async ({
      id,
      action,
    }: {
      id: string;
      action: "hide" | "publish" | "delete" | "pin" | "unpin" | "feature" | "unfeature";
    }) => {
      if (action === "delete") return deleteAdminTopic(id);
      if (action === "hide") return updateAdminTopic(id, { status: "hidden" });
      if (action === "publish") return updateAdminTopic(id, { status: "published" });
      if (action === "pin") return updateAdminTopic(id, { is_pinned: true });
      if (action === "unpin") return updateAdminTopic(id, { is_pinned: false });
      if (action === "feature") return updateAdminTopic(id, { is_featured: true });
      return updateAdminTopic(id, { is_featured: false });
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "topics"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (topics.isPending) return <QueryLoading label="正在加载帖子" />;
  if (topics.isError) return <QueryError message="无法加载帖子列表" />;

  return (
    <div>
      <AdminPageHeader title="帖子管理" description="审核、隐藏、置顶与精华。" />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索标题 / slug"
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
          <option value="hidden">已隐藏</option>
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
      <AdminTable headers={["帖子", "作者", "状态", "统计", "时间", "操作"]}>
        {topics.data.items.map((topic) => (
          <tr key={topic.id}>
            <td className="px-3 py-3">
              <div className="font-medium">{topic.title}</div>
              <div className="text-muted-foreground">
                {topic.category_name} · /{topic.slug}
              </div>
            </td>
            <td className="px-3 py-3">@{topic.author_username}</td>
            <td className="px-3 py-3">
              {topic.status}
              {topic.is_pinned ? " · 置顶" : ""}
              {topic.is_featured ? " · 精华" : ""}
            </td>
            <td className="px-3 py-3 text-muted-foreground">
              浏览 {topic.view_count} / 回复 {topic.reply_count}
            </td>
            <td className="px-3 py-3">{formatDateTime(topic.created_at)}</td>
            <td className="px-3 py-3">
              <div className="flex flex-wrap gap-2">
                {topic.status === "published" ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => mutation.mutate({ id: topic.id, action: "hide" })}
                  >
                    隐藏
                  </Button>
                ) : topic.status === "hidden" ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => mutation.mutate({ id: topic.id, action: "publish" })}
                  >
                    恢复
                  </Button>
                ) : null}
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    mutation.mutate({
                      id: topic.id,
                      action: topic.is_pinned ? "unpin" : "pin",
                    })
                  }
                >
                  {topic.is_pinned ? "取消置顶" : "置顶"}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    mutation.mutate({
                      id: topic.id,
                      action: topic.is_featured ? "unfeature" : "feature",
                    })
                  }
                >
                  {topic.is_featured ? "取消精华" : "精华"}
                </Button>
                {topic.status !== "deleted" ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      if (window.confirm(`确认删除帖子「${topic.title}」？`)) {
                        mutation.mutate({ id: topic.id, action: "delete" });
                      }
                    }}
                  >
                    删除
                  </Button>
                ) : null}
              </div>
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={topics.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
