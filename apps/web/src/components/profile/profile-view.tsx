"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import type { ProfileUpdateRequest, User } from "@lumiforum/types";
import { Alert, Avatar, AvatarFallback, AvatarImage, Button, Input, Label } from "@lumiforum/ui";
import { useMutation } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { Brand } from "@/components/brand";
import { LoadingIndicator } from "@/components/loading-indicator";
import { AvatarUpload } from "@/components/uploads/avatar-upload";
import { bindSteam, errorMessage, syncSteam, unbindSteam, updateProfile } from "@/lib/api/auth";
import { profileSchema, type ProfileFormValues } from "@/lib/auth/schemas";

export function ProfileView() {
  const { user, signOut, setCurrentUser } = useAuth();
  const logoutMutation = useMutation({
    mutationFn: signOut,
  });

  if (!user) {
    return (
      <PageFrame onLogout={() => logoutMutation.mutate()} loggingOut={logoutMutation.isPending}>
        <div className="flex min-h-[50vh] items-center justify-center text-muted-foreground">
          <LoadingIndicator className="mr-2 size-5" />
          正在载入账户
        </div>
      </PageFrame>
    );
  }

  return (
    <PageFrame onLogout={() => logoutMutation.mutate()} loggingOut={logoutMutation.isPending}>
      <ProfileContent user={user} onUpdated={setCurrentUser} />
    </PageFrame>
  );
}

function PageFrame({
  children,
  onLogout,
  loggingOut,
}: {
  children: React.ReactNode;
  onLogout: () => void;
  loggingOut: boolean;
}) {
  return (
    <div className="min-h-screen bg-surface">
      <header className="border-b border-border bg-white">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-5 sm:px-8">
          <Brand />
          <Button
            variant="ghost"
            size="sm"
            className="gap-2"
            onClick={onLogout}
            disabled={loggingOut}
          >
            {loggingOut ? <LoadingIndicator /> : null}
            退出
          </Button>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-5 py-10 sm:px-8">{children}</main>
    </div>
  );
}

function ProfileContent({ user, onUpdated }: { user: User; onUpdated: (user: User) => void }) {
  return (
    <div>
      <div className="mb-10 flex flex-col justify-between gap-5 border-b border-border pb-8 sm:flex-row sm:items-end">
        <div className="flex items-center gap-4">
          <Avatar className="size-16 border border-border">
            {user.avatar ? <AvatarImage src={user.avatar} alt="" /> : null}
            <AvatarFallback>{initials(user)}</AvatarFallback>
          </Avatar>
          <div>
            <p className="text-sm text-muted-foreground">个人中心</p>
            <h1 className="mt-1 text-3xl font-semibold">{user.nickname || user.username}</h1>
            <div className="mt-3 flex flex-wrap gap-4 text-sm text-muted-foreground">
              <Link href={`/users/${user.id}/followers`} className="hover:text-foreground">
                <span className="font-medium text-foreground">{user.followers_count}</span> 粉丝
              </Link>
              <Link href={`/users/${user.id}/following`} className="hover:text-foreground">
                <span className="font-medium text-foreground">{user.following_count}</span> 关注
              </Link>
              <Link href={`/users/${user.id}/topics`} className="hover:text-foreground">
                我发布的帖子
              </Link>
              <Link href="/favorites" className="hover:text-foreground">
                我的收藏
              </Link>
              <Link href="/notifications" className="hover:text-foreground">
                通知中心
              </Link>
            </div>
          </div>
        </div>
        <div className="inline-flex items-center gap-2 text-sm text-primary">
          <span className="size-2 rounded-full bg-primary" aria-hidden="true" />
          {user.role.name}
        </div>
      </div>

      <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_320px]">
        <section aria-labelledby="profile-settings-title">
          <h2 id="profile-settings-title" className="text-lg font-semibold">
            公开资料
          </h2>
          <p className="mt-1 text-sm text-muted-foreground">更新社区中展示的身份信息。</p>
          <ProfileEditor key={user.updated_at} user={user} onUpdated={onUpdated} />
        </section>

        <aside className="border-t border-border pt-8 lg:border-l lg:border-t-0 lg:pl-8 lg:pt-0">
          <h2 className="text-lg font-semibold">账户信息</h2>
          <dl className="mt-5 divide-y divide-border border-y border-border text-sm">
            <AccountRow label="用户名" value={user.username} />
            <AccountRow label="邮箱" value={user.email} />
            <AccountRow label="状态" value={statusLabel(user.status)} />
            <AccountRow label="邮箱验证" value={user.email_verified ? "已验证" : "未验证"} />
            <AccountRow label="粉丝" value={String(user.followers_count)} />
            <AccountRow label="关注" value={String(user.following_count)} />
            <AccountRow label="加入时间" value={formatDate(user.created_at)} />
          </dl>
          <SteamAccount user={user} onUpdated={onUpdated} />
        </aside>
      </div>
    </div>
  );
}

function ProfileEditor({ user, onUpdated }: { user: User; onUpdated: (user: User) => void }) {
  const form = useForm<ProfileFormValues>({
    resolver: zodResolver(profileSchema),
    defaultValues: {
      nickname: user.nickname ?? "",
    },
  });
  const mutation = useMutation({
    mutationFn: updateProfile,
    onSuccess: (updated) => {
      onUpdated(updated);
    },
    onError: (error) => form.setError("root", { message: errorMessage(error) }),
  });
  const submit = form.handleSubmit((values) => {
    const patch: ProfileUpdateRequest = {};
    if (form.formState.dirtyFields.nickname) {
      patch.nickname = values.nickname || null;
    }
    if (Object.keys(patch).length > 0) {
      mutation.mutate(patch);
    }
  });

  return (
    <div className="max-w-xl">
      <AvatarUpload user={user} onUpdated={onUpdated} />
      <form className="mt-6 space-y-5" onSubmit={submit}>
        {form.formState.errors.root?.message ? (
          <Alert>{form.formState.errors.root.message}</Alert>
        ) : null}

        <div className="space-y-2">
          <Label htmlFor="profile-nickname">昵称</Label>
          <Input
            id="profile-nickname"
            aria-invalid={Boolean(form.formState.errors.nickname)}
            {...form.register("nickname")}
          />
          <p className="min-h-5 text-sm text-destructive">
            {form.formState.errors.nickname?.message}
          </p>
        </div>

        <Button
          type="submit"
          className="gap-2"
          disabled={mutation.isPending || !form.formState.isDirty}
        >
          {mutation.isPending ? <LoadingIndicator /> : null}
          {mutation.isPending ? "正在保存" : "保存修改"}
        </Button>
      </form>
    </div>
  );
}

function SteamAccount({ user, onUpdated }: { user: User; onUpdated: (user: User) => void }) {
  const [password, setPassword] = useState("");
  const [confirmingUnbind, setConfirmingUnbind] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bindMutation = useMutation({
    mutationFn: bindSteam,
    onSuccess: ({ authorization_url }) => window.location.assign(authorization_url),
    onError: (cause) => setError(errorMessage(cause)),
  });
  const unbindMutation = useMutation({
    mutationFn: unbindSteam,
    onSuccess: (updated) => {
      onUpdated(updated);
      setPassword("");
      setConfirmingUnbind(false);
      setError(null);
    },
    onError: (cause) => setError(errorMessage(cause)),
  });
  const syncMutation = useMutation({
    mutationFn: syncSteam,
    onSuccess: onUpdated,
    onError: (cause) => setError(errorMessage(cause)),
  });
  const pending = bindMutation.isPending || unbindMutation.isPending || syncMutation.isPending;

  return (
    <section className="mt-8" aria-labelledby="steam-account-title">
      <h3 id="steam-account-title" className="font-semibold">
        Steam 账户
      </h3>
      {error ? <Alert className="mt-4">{error}</Alert> : null}
      {user.steam_id ? (
        <div className="mt-4 space-y-4 text-sm">
          <div className="flex items-center gap-3">
            <Avatar className="size-10 border border-border">
              {user.steam_avatar_medium ? (
                <AvatarImage src={user.steam_avatar_medium} alt="" />
              ) : null}
              <AvatarFallback>ST</AvatarFallback>
            </Avatar>
            <div className="min-w-0">
              <p className="truncate font-medium">{user.steam_persona_name || user.steam_id}</p>
              <p className="text-xs text-muted-foreground">
                SteamID64: {user.steam_id}
                {user.steam_country_code ? ` · ${user.steam_country_code}` : ""}
              </p>
              {user.steam_profile_url ? (
                <a
                  href={user.steam_profile_url}
                  target="_blank"
                  rel="noreferrer"
                  className="text-muted-foreground hover:text-foreground"
                >
                  查看 Steam 资料
                </a>
              ) : null}
            </div>
          </div>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              disabled={pending}
              onClick={() => {
                setError(null);
                syncMutation.mutate();
              }}
            >
              {syncMutation.isPending ? "同步中" : "同步资料"}
            </Button>
            {user.has_password ? (
              <Button
                size="sm"
                variant="ghost"
                disabled={pending}
                onClick={() => setConfirmingUnbind((value) => !value)}
              >
                解除绑定
              </Button>
            ) : null}
          </div>
          {!user.has_password ? (
            <p className="text-muted-foreground">
              Steam 是当前唯一登录方式，请先设置密码再解除绑定。
            </p>
          ) : null}
          {user.has_password && confirmingUnbind ? (
            <div className="space-y-3 border-t border-border pt-4">
              <Label htmlFor="steam-unbind-password">输入当前密码确认</Label>
              <Input
                id="steam-unbind-password"
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
              />
              <Button
                size="sm"
                disabled={!password || pending}
                onClick={() => {
                  setError(null);
                  unbindMutation.mutate({ password });
                }}
              >
                {unbindMutation.isPending ? "正在解绑" : "确认解绑"}
              </Button>
            </div>
          ) : null}
        </div>
      ) : (
        <div className="mt-4">
          <p className="text-sm text-muted-foreground">尚未绑定 Steam 账户。</p>
          <Button
            size="sm"
            variant="outline"
            className="mt-3"
            disabled={pending}
            onClick={() => {
              setError(null);
              bindMutation.mutate();
            }}
          >
            {bindMutation.isPending ? "正在连接" : "绑定 Steam"}
          </Button>
        </div>
      )}
    </section>
  );
}

function AccountRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-4 py-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="break-words text-right font-medium">{value}</dd>
    </div>
  );
}

function initials(user: User): string {
  return (user.nickname || user.username).slice(0, 2).toUpperCase();
}

function statusLabel(status: User["status"]): string {
  return {
    active: "正常",
    pending: "待处理",
    suspended: "已暂停",
    disabled: "已禁用",
  }[status];
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" }).format(new Date(value));
}
