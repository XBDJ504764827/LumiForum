"use client";

import { Alert, Button } from "@lumiforum/ui";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";

import { useAuth } from "@/components/auth/auth-provider";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage } from "@/lib/api/auth";

const steamErrorMessages: Record<string, string> = {
  access_denied: "你已取消 Steam 授权。",
  steam_access_denied: "你已取消 Steam 授权。",
  account_conflict: "该 Steam 账户已绑定其他用户。",
  steam_account_conflict: "该 Steam 账户已绑定其他用户。",
  invalid_state: "Steam 授权已失效，请重新尝试。",
  steam_invalid_state: "Steam 授权已失效，请重新尝试。",
  steam_auth_failed: "Steam 认证失败，请重新尝试。",
  steam_unavailable: "Steam 登录暂时不可用，请稍后重试。",
};

export function SteamAuthComplete() {
  const router = useRouter();
  const { restoreSession } = useAuth();
  const started = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [errorDestination, setErrorDestination] = useState("/login");

  useEffect(() => {
    if (started.current) {
      return;
    }
    started.current = true;

    const params = new URLSearchParams(window.location.search);
    if (window.location.hash) {
      window.history.replaceState(null, "", `${window.location.pathname}${window.location.search}`);
    }
    const mode = params.get("mode");
    const steamError = params.get("error");
    if (steamError) {
      queueMicrotask(() => {
        setErrorDestination(mode === "bind" ? "/profile" : "/login");
        setError(steamErrorMessages[steamError] ?? "Steam 认证未完成，请重新尝试。");
      });
      return;
    }

    const destination = mode === "bind" ? "/profile" : "/";
    void restoreSession()
      .then(() => router.replace(destination))
      .catch((cause) => setError(errorMessage(cause)));
  }, [restoreSession, router]);

  return (
    <div className="flex min-h-screen items-center justify-center bg-surface px-5 py-10">
      <div className="w-full max-w-sm rounded-lg border border-border bg-white p-8 text-center">
        <h1 className="text-2xl font-semibold">Steam 认证</h1>
        {error ? (
          <div className="mt-6 space-y-5">
            <Alert>{error}</Alert>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => router.replace(errorDestination)}
            >
              {errorDestination === "/profile" ? "返回个人中心" : "返回登录"}
            </Button>
            <Link href="/" className="block text-sm text-muted-foreground hover:text-foreground">
              返回首页
            </Link>
          </div>
        ) : (
          <div className="mt-6 flex items-center justify-center text-sm text-muted-foreground">
            <LoadingIndicator className="mr-2 size-5" />
            正在恢复账户会话
          </div>
        )}
      </div>
    </div>
  );
}
