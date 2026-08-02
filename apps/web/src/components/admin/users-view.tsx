"use client";

import type { AdminUserListParams, UserStatus } from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

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
  deleteAdminUser,
  listAdminRoles,
  listAdminUsers,
  updateAdminUser,
} from "@/lib/api/admin";

export function AdminUsersView() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<AdminUserListParams>({ page: 1, page_size: 20 });
  const [q, setQ] = useState("");
  const [error, setError] = useState<string | null>(null);

  const users = useQuery({
    queryKey: adminKeys.users(params),
    queryFn: () => listAdminUsers(params),
  });
  const roles = useQuery({ queryKey: adminKeys.roles, queryFn: listAdminRoles });

  const mutation = useMutation({
    mutationFn: async ({
      id,
      status,
      role,
      remove,
    }: {
      id: string;
      status?: UserStatus;
      role?: string;
      remove?: boolean;
    }) => {
      if (remove) return deleteAdminUser(id);
      return updateAdminUser(id, { status, role });
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  const totalPages = users.data?.pagination.total_pages ?? 1;
  const roleOptions = useMemo(() => roles.data ?? [], [roles.data]);

  if (users.isPending) return <QueryLoading label="正在加载用户" />;
  if (users.isError) return <QueryError message="无法加载用户列表" />;

  return (
    <div>
      <AdminPageHeader title="用户管理" description="搜索用户、调整角色与状态。" />
      <AdminToolbar>
        <Input
          value={q}
          onChange={(event) => setQ(event.target.value)}
          placeholder="搜索用户名 / 邮箱"
          className="max-w-xs"
        />
        <Select
          value={params.status ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              status: (event.target.value || undefined) as UserStatus | undefined,
            }))
          }
        >
          <option value="">全部状态</option>
          <option value="active">正常</option>
          <option value="suspended">冻结</option>
          <option value="disabled">禁用</option>
          <option value="pending">待处理</option>
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
      <AdminTable headers={["用户", "角色", "状态", "注册时间", "最后登录", "操作"]}>
        {users.data.items.map((user) => (
          <tr key={user.id}>
            <td className="px-3 py-3">
              <div className="font-medium">
                <a href={`/admin/users/${user.id}`} className="hover:text-primary">
                  {user.nickname || user.username}
                </a>
              </div>
              <div className="text-muted-foreground">
                @{user.username} · {user.email}
              </div>
            </td>
            <td className="px-3 py-3">
              <Select
                value={user.role.code}
                disabled={mutation.isPending}
                onChange={(event) => {
                  if (
                    event.target.value !== user.role.code &&
                    window.confirm(`确认将 ${user.username} 角色修改为 ${event.target.value}？`)
                  ) {
                    mutation.mutate({ id: user.id, role: event.target.value });
                  }
                }}
              >
                {roleOptions.map((role) => (
                  <option key={role.code} value={role.code}>
                    {role.name}
                  </option>
                ))}
              </Select>
            </td>
            <td className="px-3 py-3">{user.status}</td>
            <td className="px-3 py-3">{formatDateTime(user.created_at)}</td>
            <td className="px-3 py-3">{formatDateTime(user.last_login_at)}</td>
            <td className="px-3 py-3">
              <div className="flex flex-wrap gap-2">
                {user.status === "active" ? (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      if (window.confirm(`确认冻结用户 ${user.username}？`)) {
                        mutation.mutate({ id: user.id, status: "suspended" });
                      }
                    }}
                  >
                    冻结
                  </Button>
                ) : (
                  <Button
                    type="button"
                    size="sm"
                    variant="outline"
                    onClick={() => mutation.mutate({ id: user.id, status: "active" })}
                  >
                    解封
                  </Button>
                )}
                <Button
                  type="button"
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    if (window.confirm(`确认禁用用户 ${user.username}？此为软删除。`)) {
                      mutation.mutate({ id: user.id, remove: true });
                    }
                  }}
                >
                  禁用
                </Button>
              </div>
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={params.page ?? 1}
        totalPages={totalPages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
