"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { Alert, Button, Input, Label } from "@lumiforum/ui";
import { useMutation } from "@tanstack/react-query";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useRef, useState } from "react";
import { useForm } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage, setSteamContact } from "@/lib/api/auth";
import { steamContactSchema, type SteamContactFormValues } from "@/lib/auth/schemas";

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
  const [contactRequired, setContactRequired] = useState(false);

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
      .then((user) => {
        // 首次 Steam 登录（或尚未填写联系方式）时，先引导玩家填写联系方式，
        // 填写后与当前 Steam 账户绑定，方便管理员后续追溯。
        if (mode === "login" && params.get("contact") === "required") {
          setContactRequired(true);
          return;
        }
        if (user.contact == null && mode === "login") {
          setContactRequired(true);
          return;
        }
        router.replace(destination);
      })
      .catch((cause) => setError(errorMessage(cause)));
  }, [restoreSession, router]);

  return (
    <div className="flex min-h-screen items-center justify-center bg-surface px-5 py-10">
      <div className="w-full max-w-sm rounded-lg border border-border bg-white p-8">
        <h1 className="text-2xl font-semibold">Steam 认证</h1>
        {error ? (
          <div className="mt-6 space-y-5 text-center">
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
        ) : contactRequired ? (
          <ContactForm onDone={() => router.replace("/")} onSkip={() => router.replace("/")} />
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

function ContactForm({ onDone, onSkip }: { onDone: () => void; onSkip: () => void }) {
  const { setCurrentUser } = useAuth();
  const form = useForm<SteamContactFormValues>({
    resolver: zodResolver(steamContactSchema),
    defaultValues: { contact: "" },
  });
  const mutation = useMutation({
    mutationFn: setSteamContact,
    onSuccess: (user) => {
      setCurrentUser(user);
      onDone();
    },
    onError: (error) => form.setError("contact", { message: errorMessage(error) }),
  });

  return (
    <form
      className="mt-6 space-y-4"
      onSubmit={form.handleSubmit((values) => mutation.mutate(values))}
    >
      <div>
        <p className="text-sm leading-6 text-muted-foreground">
          欢迎首次使用 Steam 登录。请填写联系方式（QQ、手机号或微信号等）， 与你的 Steam
          账户绑定，方便管理员在必要时联系与追溯。
        </p>
      </div>

      <div className="space-y-2">
        <Label htmlFor="steam-contact">联系方式</Label>
        <Input
          id="steam-contact"
          autoFocus
          autoComplete="off"
          placeholder="QQ、手机号或微信号等"
          aria-invalid={Boolean(form.formState.errors.contact)}
          {...form.register("contact")}
        />
        <p className="min-h-5 text-sm text-destructive">{form.formState.errors.contact?.message}</p>
      </div>

      <Button className="w-full gap-2" type="submit" disabled={mutation.isPending}>
        {mutation.isPending ? <LoadingIndicator /> : null}
        {mutation.isPending ? "正在保存" : "保存并进入论坛"}
      </Button>

      <button
        type="button"
        className="block w-full text-center text-xs text-muted-foreground hover:text-foreground"
        disabled={mutation.isPending}
        onClick={onSkip}
      >
        暂时跳过（下次 Steam 登录时会再次提醒）
      </button>
    </form>
  );
}
