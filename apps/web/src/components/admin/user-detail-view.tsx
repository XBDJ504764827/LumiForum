"use client";

import type { Route } from "next";
import { Button } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound, ShieldAlert } from "lucide-react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import { AdminBreadcrumb, formatDateTime } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import { adminKeys, forceAdminLogout, getAdminUserDetail, updateAdminUser } from "@/lib/api/admin";

export function AdminUserDetailView({ userId }: { userId: string }) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const detail = useQuery({
    queryKey: adminKeys.userDetail(userId),
    queryFn: () => getAdminUserDetail(userId),
  });
  const action = useMutation({
    mutationFn: async ({ kind }: { kind: "disable" | "enable" | "force-logout" }) => {
      if (kind === "force-logout") return forceAdminLogout(userId);
      return updateAdminUser(userId, { status: kind === "disable" ? "disabled" : "active" });
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: adminKeys.userDetail(userId) });
      await queryClient.invalidateQueries({ queryKey: ["admin", "users"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (detail.isPending) return <QueryLoading label="正在加载用户详情" />;
  if (detail.isError || !detail.data) return <QueryError message="用户不存在" />;

  const { user } = detail.data;

  return (
    <div>
      <AdminBreadcrumb
        items={[{ label: "用户", href: "/admin/users" }, { label: user.username }]}
      />
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}

      <div className="mb-6 flex flex-wrap items-center justify-between gap-4 border-b border-border pb-5">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {user.nickname || user.username}
            <span className="ml-2 text-base font-normal text-muted-foreground">
              @{user.username}
            </span>
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {user.email} · {user.role.name} · {user.status}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {user.status === "active" ? (
            <Button
              size="sm"
              variant="outline"
              disabled={action.isPending}
              onClick={() => {
                if (window.confirm(`确定禁用用户 ${user.username} 吗？`))
                  action.mutate({ kind: "disable" });
              }}
            >
              禁用用户
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              disabled={action.isPending}
              onClick={() => action.mutate({ kind: "enable" })}
            >
              解禁用户
            </Button>
          )}
          <Button
            size="sm"
            variant="outline"
            className="gap-1.5 text-destructive hover:text-destructive"
            disabled={action.isPending}
            onClick={() => {
              if (window.confirm(`强制 ${user.username} 退出登录？（其所有会话将失效）`))
                action.mutate({ kind: "force-logout" });
            }}
          >
            <KeyRound className="size-3.5" aria-hidden="true" />
            强制退出登录
          </Button>
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-2">
        <section className="border border-border bg-white p-5">
          <h2 className="mb-4 font-semibold">基本信息</h2>
          <dl className="grid grid-cols-[110px_minmax(0,1fr)] gap-y-2.5 text-sm">
            <dt className="text-muted-foreground">用户 ID</dt>
            <dd className="break-all font-mono text-xs">{user.id}</dd>
            <dt className="text-muted-foreground">邮箱</dt>
            <dd>{user.email}</dd>
            <dt className="text-muted-foreground">Steam ID</dt>
            <dd>{detail.data.steam_id ?? "未绑定"}</dd>
            <dt className="text-muted-foreground">Steam 昵称</dt>
            <dd>{detail.data.steam_persona_name ?? "-"}</dd>
            <dt className="text-muted-foreground">角色</dt>
            <dd>{user.role.name}</dd>
            <dt className="text-muted-foreground">状态</dt>
            <dd>{user.status}</dd>
            <dt className="text-muted-foreground">邮箱验证</dt>
            <dd>{user.email_verified ? "已验证" : "未验证"}</dd>
            <dt className="text-muted-foreground">注册时间</dt>
            <dd>{formatDateTime(user.created_at)}</dd>
            <dt className="text-muted-foreground">最近登录</dt>
            <dd>{user.last_login_at ? formatDateTime(user.last_login_at) : "从未登录"}</dd>
          </dl>
        </section>

        <section className="border border-border bg-white p-5">
          <h2 className="mb-4 font-semibold">内容统计</h2>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Stat label="登录次数" value={detail.data.login_count} />
            <Stat label="发帖" value={detail.data.topics_count} />
            <Stat label="评论" value={detail.data.comments_count} />
            <Stat label="举报提交" value={detail.data.reports_made} />
            <Stat label="有效处罚" value={detail.data.sanctions_active} />
            <Stat label="粉丝" value={user.followers_count} />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <Link
              href={`/users/${userId}/topics`}
              target="_blank"
              rel="noreferrer"
              className="inline-flex h-9 items-center rounded-md border border-border px-3 text-sm hover:bg-muted"
            >
              查看 TA 的帖子
            </Link>
            <Button
              size="sm"
              variant="outline"
              onClick={() => router.push(`/admin/users/${userId}/sanctions` as Route)}
            >
              <ShieldAlert className="mr-1.5 size-3.5" aria-hidden="true" />
              处罚记录
            </Button>
          </div>
        </section>

        <section className="border border-border bg-white p-5 xl:col-span-2">
          <h2 className="mb-4 font-semibold">最近登录记录</h2>
          {detail.data.recent_logins.length === 0 ? (
            <p className="text-sm text-muted-foreground">暂无登录记录</p>
          ) : (
            <div className="divide-y divide-border text-sm">
              {detail.data.recent_logins.map((record) => (
                <div
                  key={record.id}
                  className="flex flex-wrap items-center justify-between gap-2 py-2.5"
                >
                  <span className="tabular-nums">{formatDateTime(record.created_at)}</span>
                  <span className="text-muted-foreground">
                    IP: {record.ip ?? "未知"}
                    {record.revoked_at ? " · 已失效" : ""}
                  </span>
                  <span className="max-w-md truncate text-xs text-muted-foreground">
                    {record.user_agent ?? ""}
                  </span>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-md bg-muted/50 px-3 py-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 text-xl font-semibold tabular-nums">{value}</p>
    </div>
  );
}
