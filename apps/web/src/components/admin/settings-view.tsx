"use client";

import { Alert, Button, Input, Label } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CircleAlert, Save } from "lucide-react";
import { useState } from "react";

import { AdminPageHeader } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import { adminKeys, getAdminSettings, updateAdminSettings } from "@/lib/api/admin";

export function AdminSettingsView() {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);

  const settings = useQuery({
    queryKey: adminKeys.settings,
    queryFn: getAdminSettings,
  });
  if (settings.data && Object.keys(draft).length === 0) {
    const next: Record<string, string> = {};
    for (const item of settings.data) {
      next[item.key] = String(item.value);
    }
    setDraft(next);
  }

  const save = useMutation({
    mutationFn: () => {
      const entries = settings.data?.map((item) => {
        const raw = draft[item.key] ?? String(item.value);
        if (item.key === "upload_max_bytes") return { key: item.key, value: Number(raw) || 0 };
        if (
          [
            "registration_enabled",
            "topic_create_enabled",
            "comment_enabled",
            "upload_enabled",
          ].includes(item.key)
        ) {
          return { key: item.key, value: raw === "true" };
        }
        return { key: item.key, value: raw };
      });
      return updateAdminSettings(entries ?? []);
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: adminKeys.settings });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (settings.isPending) return <QueryLoading label="正在加载设置" />;
  if (settings.isError || !settings.data) return <QueryError message="设置加载失败" />;

  const byKey = (key: string) => settings.data.find((item) => item.key === key);

  return (
    <div>
      <AdminPageHeader
        title="系统设置"
        description="论坛全局配置，保存后立即生效（公开接口 60 秒内同步）。"
      />
      {error ? (
        <Alert className="mb-5">
          <CircleAlert className="size-4 shrink-0" aria-hidden="true" />
          {error}
        </Alert>
      ) : null}

      <div className="max-w-2xl space-y-5">
        <section className="border border-border bg-white p-5">
          <h2 className="mb-4 font-semibold">论坛信息</h2>
          <div className="space-y-4">
            <div>
              <Label htmlFor="site_name">论坛名称</Label>
              <Input
                id="site_name"
                className="mt-1.5"
                value={draft.site_name ?? ""}
                onChange={(event) => setDraft((d) => ({ ...d, site_name: event.target.value }))}
              />
              <p className="mt-1 text-xs text-muted-foreground">
                {byKey("site_name")?.description}
              </p>
            </div>
            <div>
              <Label htmlFor="site_description">论坛描述</Label>
              <Input
                id="site_description"
                className="mt-1.5"
                value={draft.site_description ?? ""}
                onChange={(event) =>
                  setDraft((d) => ({ ...d, site_description: event.target.value }))
                }
              />
            </div>
          </div>
        </section>

        <section className="border border-border bg-white p-5">
          <h2 className="mb-4 font-semibold">功能开关</h2>
          <div className="grid gap-3 sm:grid-cols-2">
            <Toggle
              label="开放注册"
              hint={byKey("registration_enabled")?.description ?? undefined}
              checked={draft.registration_enabled === "true"}
              onChange={(value) => setDraft((d) => ({ ...d, registration_enabled: String(value) }))}
            />
            <Toggle
              label="允许发帖"
              hint={byKey("topic_create_enabled")?.description ?? undefined}
              checked={draft.topic_create_enabled === "true"}
              onChange={(value) => setDraft((d) => ({ ...d, topic_create_enabled: String(value) }))}
            />
            <Toggle
              label="允许评论"
              hint={byKey("comment_enabled")?.description ?? undefined}
              checked={draft.comment_enabled === "true"}
              onChange={(value) => setDraft((d) => ({ ...d, comment_enabled: String(value) }))}
            />
            <Toggle
              label="允许上传"
              hint={byKey("upload_enabled")?.description ?? undefined}
              checked={draft.upload_enabled === "true"}
              onChange={(value) => setDraft((d) => ({ ...d, upload_enabled: String(value) }))}
            />
          </div>
        </section>

        <section className="border border-border bg-white p-5">
          <h2 className="mb-4 font-semibold">上传限制</h2>
          <div>
            <Label htmlFor="upload_max_bytes">单文件大小上限（字节）</Label>
            <Input
              id="upload_max_bytes"
              type="number"
              min={1024}
              className="mt-1.5"
              value={draft.upload_max_bytes ?? ""}
              onChange={(event) =>
                setDraft((d) => ({ ...d, upload_max_bytes: event.target.value }))
              }
            />
            <p className="mt-1 text-xs text-muted-foreground">
              当前 {(Number(draft.upload_max_bytes) / 1024 / 1024).toFixed(1)} MB
            </p>
          </div>
        </section>

        <Button className="gap-2" disabled={save.isPending} onClick={() => save.mutate()}>
          <Save className="size-4" aria-hidden="true" />
          {save.isPending ? "保存中…" : "保存设置"}
        </Button>
      </div>
    </div>
  );
}

function Toggle({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-start justify-between gap-3 rounded-md border border-border px-3 py-3">
      <span>
        <span className="block text-sm font-medium">{label}</span>
        {hint ? <span className="block text-xs text-muted-foreground">{hint}</span> : null}
      </span>
      <input
        type="checkbox"
        className="mt-0.5 size-4 accent-primary"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}
