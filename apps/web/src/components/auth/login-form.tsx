"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { Alert, Button, Input, Label } from "@lumiforum/ui";
import { useMutation } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage, steamLoginUrl } from "@/lib/api/auth";
import { loginSchema, type LoginFormValues } from "@/lib/auth/schemas";

export function LoginForm() {
  const { signIn } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const form = useForm<LoginFormValues>({
    resolver: zodResolver(loginSchema),
    defaultValues: { identifier: "", password: "" },
  });
  const mutation = useMutation({
    mutationFn: signIn,
    onError: (error) => {
      form.setError("root", { message: errorMessage(error) });
    },
  });

  return (
    <div>
      <div className="mb-8">
        <h2 className="text-3xl font-semibold">登录</h2>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">继续进入你的社区账户。</p>
      </div>

      <form className="space-y-5" onSubmit={form.handleSubmit((values) => mutation.mutate(values))}>
        {form.formState.errors.root?.message ? (
          <Alert>{form.formState.errors.root.message}</Alert>
        ) : null}

        <div className="space-y-2">
          <Label htmlFor="identifier">用户名或邮箱</Label>
          <Input
            id="identifier"
            autoComplete="username"
            autoFocus
            aria-invalid={Boolean(form.formState.errors.identifier)}
            {...form.register("identifier")}
          />
          <p className="min-h-5 text-sm text-destructive">
            {form.formState.errors.identifier?.message}
          </p>
        </div>

        <div className="space-y-2">
          <Label htmlFor="password">密码</Label>
          <div className="relative">
            <Input
              id="password"
              type={showPassword ? "text" : "password"}
              autoComplete="current-password"
              className="pr-11"
              aria-invalid={Boolean(form.formState.errors.password)}
              {...form.register("password")}
            />
            <button
              type="button"
              className="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-muted-foreground hover:text-foreground"
              aria-label={showPassword ? "隐藏密码" : "显示密码"}
              title={showPassword ? "隐藏密码" : "显示密码"}
              onClick={() => setShowPassword((value) => !value)}
            >
              <span className="text-xs">{showPassword ? "隐藏" : "显示"}</span>
            </button>
          </div>
          <p className="min-h-5 text-sm text-destructive">
            {form.formState.errors.password?.message}
          </p>
        </div>

        <Button className="w-full gap-2" type="submit" disabled={mutation.isPending}>
          {mutation.isPending ? <LoadingIndicator /> : null}
          {mutation.isPending ? "正在登录" : "登录"}
        </Button>
      </form>

      <div className="my-6 flex items-center gap-3 text-xs text-muted-foreground">
        <span className="h-px flex-1 bg-border" />
        或
        <span className="h-px flex-1 bg-border" />
      </div>
      <Button
        variant="outline"
        className="w-full"
        onClick={() => window.location.assign(steamLoginUrl())}
      >
        使用 Steam 登录
      </Button>
      <p className="mt-2 text-center text-xs text-muted-foreground">
        无需注册，首次登录会自动创建论坛账户
      </p>

      <p className="mt-7 text-center text-sm text-muted-foreground">
        还没有账户？{" "}
        <Link href="/register" className="font-medium text-primary hover:underline">
          创建账户
        </Link>
      </p>
    </div>
  );
}
