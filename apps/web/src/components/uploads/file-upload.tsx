"use client";

import type { Upload, UploadCategory } from "@lumiforum/types";
import { Alert, Button } from "@lumiforum/ui";
import { ImagePlus, RotateCcw, UploadCloud } from "lucide-react";
import { useId, useState } from "react";

import { errorMessage } from "@/lib/api/errors";
import { uploadFile } from "@/lib/api/uploads";
import { ATTACHMENT_HINT } from "@/components/uploads/accept";

interface Props {
  category: Exclude<UploadCategory, "avatar">;
  accept: string;
  maxBytes: number;
  onUploaded: (upload: Upload) => void;
}

export function FileUpload({ category, accept, maxBytes, onUploaded }: Props) {
  const inputId = useId();
  const [pendingFile, setPendingFile] = useState<File | null>(null);
  const [progress, setProgress] = useState(0);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startUpload = async (file: File) => {
    setPendingFile(file);
    setError(null);
    setProgress(0);
    if (file.size > maxBytes) {
      setError(`文件不能超过 ${formatBytes(maxBytes)}`);
      return;
    }
    setUploading(true);
    try {
      const result = await uploadFile(file, {
        category,
        onProgress: setProgress,
      });
      if (!("url" in result)) throw new Error("unexpected upload response");
      onUploaded(result);
      setPendingFile(null);
      setProgress(100);
    } catch (uploadError) {
      setError(errorMessage(uploadError));
    } finally {
      setUploading(false);
    }
  };

  return (
    <div className="space-y-2">
      <div
        className="flex min-h-24 items-center justify-center border border-dashed border-border bg-muted/30 px-4 py-3 transition-colors hover:border-primary/60"
        onDragOver={(event) => event.preventDefault()}
        onDrop={(event) => {
          event.preventDefault();
          const file = event.dataTransfer.files[0];
          if (file) void startUpload(file);
        }}
      >
        <input
          id={inputId}
          type="file"
          className="sr-only"
          accept={accept}
          disabled={uploading}
          onChange={(event) => {
            const file = event.target.files?.[0];
            if (file) void startUpload(file);
            event.currentTarget.value = "";
          }}
        />
        <label
          htmlFor={inputId}
          className="flex cursor-pointer items-center gap-3 text-sm text-muted-foreground"
        >
          {category === "attachment" ? (
            <UploadCloud className="size-5 text-primary" aria-hidden="true" />
          ) : (
            <ImagePlus className="size-5 text-primary" aria-hidden="true" />
          )}
          <span>{uploading ? `正在上传 ${progress}%` : "拖入文件或点击选择"}</span>
        </label>
      </div>
      {category === "attachment" ? (
        <p className="text-xs text-muted-foreground">{ATTACHMENT_HINT}</p>
      ) : null}
      {uploading ? (
        <div className="h-1 overflow-hidden bg-muted" aria-label={`上传进度 ${progress}%`}>
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
              className="shrink-0 gap-1.5"
              onClick={() => void startUpload(pendingFile)}
            >
              <RotateCcw className="size-4" aria-hidden="true" />
              重试
            </Button>
          ) : null}
        </Alert>
      ) : null}
    </div>
  );
}

function formatBytes(bytes: number): string {
  return `${Math.round(bytes / 1024 / 1024)} MB`;
}
