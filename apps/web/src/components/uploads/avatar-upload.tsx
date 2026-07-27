"use client";

import type { User } from "@lumiforum/types";
import { Alert, Button } from "@lumiforum/ui";
import { Camera, RotateCcw, Trash2 } from "lucide-react";
import { useId, useState } from "react";

import { errorMessage } from "@/lib/api/errors";
import { deleteAvatar, uploadFile } from "@/lib/api/uploads";

export function AvatarUpload({ user, onUpdated }: { user: User; onUpdated: (user: User) => void }) {
  const inputId = useId();
  const [pendingFile, setPendingFile] = useState<File | null>(null);
  const [progress, setProgress] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const upload = async (file: File) => {
    setPendingFile(file);
    setError(null);
    if (file.size > 5 * 1024 * 1024) {
      setError("头像不能超过 5 MB");
      return;
    }
    setBusy(true);
    setProgress(0);
    try {
      const result = await uploadFile(file, {
        avatar: true,
        category: "avatar",
        onProgress: setProgress,
      });
      if (!("role" in result)) throw new Error("unexpected avatar response");
      onUpdated(result);
      setPendingFile(null);
    } catch (uploadError) {
      setError(errorMessage(uploadError));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    setBusy(true);
    setError(null);
    try {
      onUpdated(await deleteAvatar());
    } catch (deleteError) {
      setError(errorMessage(deleteError));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="mt-7 space-y-3 border-b border-border pb-6">
      <div className="flex flex-wrap items-center gap-2">
        <input
          id={inputId}
          type="file"
          accept="image/jpeg,image/png,image/webp"
          className="sr-only"
          disabled={busy}
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void upload(file);
            event.currentTarget.value = "";
          }}
        />
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-2"
          disabled={busy}
          onClick={() => document.getElementById(inputId)?.click()}
        >
          <Camera className="size-4" aria-hidden="true" />
          {busy && progress > 0 ? `上传 ${progress}%` : "更换头像"}
        </Button>
        {user.avatar ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="gap-2 text-destructive"
            disabled={busy}
            onClick={() => void remove()}
          >
            <Trash2 className="size-4" aria-hidden="true" />
            删除头像
          </Button>
        ) : null}
      </div>
      {busy && progress > 0 ? (
        <div className="h-1 max-w-sm overflow-hidden bg-muted">
          <div className="h-full bg-primary transition-[width]" style={{ width: `${progress}%` }} />
        </div>
      ) : null}
      {error ? (
        <Alert className="flex items-center justify-between gap-3">
          <span>{error}</span>
          {pendingFile ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => void upload(pendingFile)}
            >
              <RotateCcw className="mr-1.5 size-4" aria-hidden="true" />
              重试
            </Button>
          ) : null}
        </Alert>
      ) : null}
    </div>
  );
}
