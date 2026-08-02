"use client";

import type { Category } from "@lumiforum/types";
import { Button, Input } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import { AdminPageHeader, AdminTable } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import {
  adminKeys,
  createAdminCategory,
  deleteAdminCategory,
  listAdminCategories,
  updateAdminCategory,
} from "@/lib/api/admin";

type CategoryAction =
  | { type: "create"; name: string }
  | { type: "toggle"; category: Category }
  | { type: "toggle-restricted"; category: Category }
  | { type: "toggle-anonymous"; category: Category }
  | { type: "sort"; category: Category; sort_order: number }
  | { type: "delete"; id: string };

export function AdminCategoriesView() {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const categories = useQuery({
    queryKey: adminKeys.categories,
    queryFn: listAdminCategories,
  });
  const mutation = useMutation({
    mutationFn: async (action: CategoryAction) => {
      if (action.type === "create") {
        return createAdminCategory({ name: action.name });
      }
      if (action.type === "toggle") {
        return updateAdminCategory(action.category.id, {
          is_visible: !action.category.is_visible,
        });
      }
      if (action.type === "toggle-restricted") {
        return updateAdminCategory(action.category.id, {
          restricted_posting: !action.category.restricted_posting,
        });
      }
      if (action.type === "toggle-anonymous") {
        return updateAdminCategory(action.category.id, {
          allow_anonymous: !action.category.allow_anonymous,
        });
      }
      if (action.type === "sort") {
        return updateAdminCategory(action.category.id, { sort_order: action.sort_order });
      }
      return deleteAdminCategory(action.id);
    },
    onSuccess: async () => {
      setError(null);
      setName("");
      await queryClient.invalidateQueries({ queryKey: adminKeys.categories });
      await queryClient.invalidateQueries({ queryKey: ["forum", "categories"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (categories.isPending) return <QueryLoading label="正在加载分类" />;
  if (categories.isError) return <QueryError message="无法加载分类" />;

  return (
    <div>
      <AdminPageHeader
        title="分类管理"
        description="自定义板块：创建、排序、可见性、发帖限制与匿名发帖开关。"
        actions={
          <div className="flex gap-2">
            <Input
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="新板块名称"
              className="w-48"
            />
            <Button
              type="button"
              disabled={!name.trim() || mutation.isPending}
              onClick={() => mutation.mutate({ type: "create", name: name.trim() })}
            >
              创建板块
            </Button>
          </div>
        }
      />
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      <AdminTable headers={["名称", "Slug", "排序", "可见", "限制", "匿名", "帖子数", "操作"]}>
        {categories.data.map((category) => (
          <tr key={category.id}>
            <td className="px-3 py-3 font-medium">{category.name}</td>
            <td className="px-3 py-3 text-muted-foreground">{category.slug}</td>
            <td className="px-3 py-3">
              <Input
                type="number"
                className="w-24"
                defaultValue={category.sort_order}
                onBlur={(event) => {
                  const sort_order = Number(event.target.value);
                  if (!Number.isNaN(sort_order) && sort_order !== category.sort_order) {
                    mutation.mutate({ type: "sort", category, sort_order });
                  }
                }}
              />
            </td>
            <td className="px-3 py-3">{category.is_visible ? "显示" : "隐藏"}</td>
            <td className="px-3 py-3">
              {category.restricted_posting ? (
                <span className="text-amber-600">仅管理员</span>
              ) : (
                <span className="text-muted-foreground">所有人</span>
              )}
            </td>
            <td className="px-3 py-3">
              {category.allow_anonymous ? (
                <span className="text-emerald-600">允许</span>
              ) : (
                <span className="text-muted-foreground">不允许</span>
              )}
            </td>
            <td className="px-3 py-3">{category.topic_count}</td>
            <td className="px-3 py-3">
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => mutation.mutate({ type: "toggle", category })}
                >
                  {category.is_visible ? "隐藏" : "显示"}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => mutation.mutate({ type: "toggle-restricted", category })}
                  title="切换是否仅管理员可发帖"
                >
                  {category.restricted_posting ? "解除发帖限制" : "设为仅管理员"}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => mutation.mutate({ type: "toggle-anonymous", category })}
                  title="切换是否允许匿名发帖"
                >
                  {category.allow_anonymous ? "关闭匿名" : "允许匿名"}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    if (window.confirm(`确认删除分类「${category.name}」？`)) {
                      mutation.mutate({ type: "delete", id: category.id });
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
    </div>
  );
}
