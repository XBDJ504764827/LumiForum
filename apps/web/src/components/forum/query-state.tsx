import { Alert } from "@lumiforum/ui";
import { CircleAlert } from "lucide-react";

import { LoadingIndicator } from "@/components/loading-indicator";

export function QueryLoading({ label = "正在加载" }: { label?: string }) {
  return (
    <div className="flex min-h-48 items-center justify-center text-sm text-muted-foreground">
      <LoadingIndicator className="mr-2" />
      {label}
    </div>
  );
}

export function QueryError({ message = "内容加载失败，请稍后重试" }: { message?: string }) {
  return (
    <Alert className="my-6">
      <CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      {message}
    </Alert>
  );
}
