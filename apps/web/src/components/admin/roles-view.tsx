"use client";

import { Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";

import { AdminPageHeader } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import {
  adminKeys,
  getAdminRolePermissions,
  listAdminPermissions,
  updateAdminRolePermissions,
} from "@/lib/api/admin";

const roles = ["user", "moderator", "administrator"];

export function AdminRolesView() {
  const queryClient = useQueryClient();
  const [selected, setSelected] = useState<string>("moderator");
  const [draft, setDraft] = useState<Set<string>>(new Set());
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const permissions = useQuery({
    queryKey: adminKeys.permissions,
    queryFn: listAdminPermissions,
  });
  const roleView = useQuery({
    queryKey: adminKeys.rolePermissions(selected),
    queryFn: () => getAdminRolePermissions(selected),
  });

  const grouped = useMemo(() => {
    const groups = new Map<string, typeof permissions.data>();
    for (const permission of permissions.data ?? []) {
      const list = groups.get(permission.group) ?? [];
      list.push(permission);
      groups.set(permission.group, list);
    }
    return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [permissions]);

  const applyRemote = (codes: string[]) => {
    setDraft(new Set(codes));
    setDirty(false);
  };
  if (roleView.data && !dirty && !roleView.isFetching) {
    // Keep draft in sync with the fetched role until the user edits.
    const current = roleView.data.permissions;
    if (draft.size === 0 || !draftSynced(draft, current)) applyRemote(current);
  }

  const save = useMutation({
    mutationFn: () => updateAdminRolePermissions(selected, { permission_codes: [...draft] }),
    onSuccess: (view) => {
      setDraft(new Set(view.permissions));
      setDirty(false);
      setError(null);
      void queryClient.invalidateQueries({ queryKey: adminKeys.rolePermissions(selected) });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  const toggle = (code: string) => {
    setDirty(true);
    setDraft((current) => {
      const next = new Set(current);
      if (next.has(code)) {
        next.delete(code);
      } else {
        next.add(code);
      }
      return next;
    });
  };

  if (permissions.isPending || roleView.isPending) return <QueryLoading label="正在加载权限数据" />;
  if (permissions.isError || roleView.isError) return <QueryError message="权限数据加载失败" />;

  return (
    <div>
      <AdminPageHeader
        title="角色权限"
        description="为各角色动态配置权限（超级管理员不可修改）。"
      />

      <div className="mb-5 flex flex-wrap items-center gap-2">
        {roles.map((code) => (
          <Button
            key={code}
            type="button"
            size="sm"
            variant={selected === code ? "default" : "outline"}
            onClick={() => {
              setSelected(code);
              setDirty(false);
              setError(null);
            }}
          >
            {code}
          </Button>
        ))}
        <span className="ml-auto text-xs text-muted-foreground">
          已选 {draft.size} 项{dirty ? " · 有未保存修改" : ""}
        </span>
      </div>

      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}

      <div className="grid gap-4 lg:grid-cols-2">
        {grouped.map(([group, items]) => {
          if (!items) return null;
          return (
            <section key={group} className="border border-border bg-white p-4">
              <h3 className="mb-3 text-sm font-semibold uppercase tracking-wide text-muted-foreground">
                {group}
              </h3>
              <ul className="grid gap-1.5 sm:grid-cols-2">
                {items.map((permission) => (
                  <li key={permission.code}>
                    <label className="flex cursor-pointer items-start gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted/50">
                      <input
                        type="checkbox"
                        className="mt-0.5 size-4 accent-primary"
                        checked={draft.has(permission.code)}
                        onChange={() => toggle(permission.code)}
                      />
                      <span>
                        <span className="block font-medium">{permission.name}</span>
                        <span className="block break-all font-mono text-xs text-muted-foreground">
                          {permission.code}
                        </span>
                      </span>
                    </label>
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </div>

      <div className="mt-6 flex items-center gap-3">
        <Button type="button" disabled={!dirty || save.isPending} onClick={() => save.mutate()}>
          {save.isPending ? "保存中…" : "保存权限配置"}
        </Button>
        {dirty ? (
          <Button
            type="button"
            variant="ghost"
            onClick={() => roleView.data && applyRemote(roleView.data.permissions)}
          >
            撤销修改
          </Button>
        ) : null}
        <span className="text-xs text-muted-foreground">
          保存后该角色所有用户的权限缓存将立即刷新
        </span>
      </div>
    </div>
  );
}

function draftSynced(draft: Set<string>, remote: string[]): boolean {
  if (draft.size !== remote.length) return false;
  return remote.every((code) => draft.has(code));
}
