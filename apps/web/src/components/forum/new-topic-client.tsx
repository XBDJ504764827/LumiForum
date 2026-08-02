"use client";

import { Alert } from "@lumiforum/ui";
import { CircleAlert } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { Suspense } from "react";

import { TopicEditor } from "@/components/forum/topic-editor";

export function NewTopicClient() {
  return (
    <Suspense>
      <PendingNotice />
      <TopicEditor mode="create" />
    </Suspense>
  );
}

function PendingNotice() {
  const searchParams = useSearchParams();
  if (searchParams.get("pending") !== "1") return null;
  return (
    <div className="mx-auto max-w-5xl px-5 pt-6 sm:px-8">
      <Alert className="border-amber-500/40 bg-amber-500/5">
        <CircleAlert className="size-4 shrink-0 text-amber-600" aria-hidden="true" />
        内容已提交，正在等待管理员审核。审核通过后将自动发布。
      </Alert>
    </div>
  );
}
