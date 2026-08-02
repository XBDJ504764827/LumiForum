"use client";

import { Button, Select } from "@lumiforum/ui";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Flag } from "lucide-react";
import { useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { errorMessage } from "@/lib/api/errors";
import { createReport } from "@/lib/api/moderation";

const reasons: Array<{ value: string; label: string }> = [
  { value: "spam", label: "垃圾广告" },
  { value: "advertisement", label: "广告推广" },
  { value: "harassment", label: "骚扰谩骂" },
  { value: "illegal_content", label: "违法违规内容" },
  { value: "cheating", label: "作弊行为" },
  { value: "copyright", label: "侵权内容" },
  { value: "other", label: "其他" },
];

export function ReportButton({
  targetType,
  targetId,
  className,
}: {
  targetType: "topic" | "comment" | "user" | "file";
  targetId: string;
  className?: string;
}) {
  const { status, user } = useAuth();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("spam");
  const [details, setDetails] = useState("");
  const [error, setError] = useState<string | null>(null);

  const mutation = useMutation({
    mutationFn: () =>
      createReport({
        target_type: targetType,
        target_id: targetId,
        reason_code: reason === "advertisement" ? "spam" : reason === "cheating" ? "other" : reason,
        details: details.trim() || undefined,
      }),
    onSuccess: () => {
      setOpen(false);
      setDetails("");
      setError(null);
      void queryClient.invalidateQueries({ queryKey: ["moderation", "my-reports"] });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (status !== "authenticated" || !user) return null;

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={
          className ??
          "inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-destructive"
        }
        title="举报"
      >
        <Flag className="size-3.5" aria-hidden="true" />
        举报
      </button>
    );
  }

  return (
    <div className="rounded-md border border-border bg-white p-3 shadow-sm">
      <p className="mb-2 text-xs font-medium">举报该{targetLabel(targetType)}</p>
      <Select
        value={reason}
        onChange={(event) => setReason(event.target.value)}
        className="h-8 text-xs"
      >
        {reasons.map((item) => (
          <option key={item.value} value={item.value}>
            {item.label}
          </option>
        ))}
      </Select>
      <textarea
        className="mt-2 min-h-16 w-full rounded-md border border-border p-2 text-xs"
        placeholder="补充说明（可选）"
        value={details}
        onChange={(event) => setDetails(event.target.value)}
      />
      {error ? <p className="mt-1 text-xs text-destructive">{error}</p> : null}
      <div className="mt-2 flex gap-2">
        <Button
          type="button"
          size="sm"
          disabled={mutation.isPending}
          onClick={() => mutation.mutate()}
        >
          {mutation.isPending ? "提交中…" : "提交举报"}
        </Button>
        <Button type="button" size="sm" variant="ghost" onClick={() => setOpen(false)}>
          取消
        </Button>
      </div>
    </div>
  );
}

function targetLabel(target: "topic" | "comment" | "user" | "file"): string {
  return { topic: "帖子", comment: "评论", user: "用户", file: "文件" }[target];
}
