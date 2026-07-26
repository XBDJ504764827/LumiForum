"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { Alert, Button, Input, Label } from "@lumiforum/ui";
import { useMutation } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import { useForm } from "react-hook-form";

import { useAuth } from "@/components/auth/auth-provider";
import { LoadingIndicator } from "@/components/loading-indicator";
import { errorMessage } from "@/lib/api/auth";
import { registerSchema, type RegisterFormValues } from "@/lib/auth/schemas";

export function RegisterForm() {
  const { signUp } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const form = useForm<RegisterFormValues>({
    resolver: zodResolver(registerSchema),
    defaultValues: {
      username: "",
      email: "",
      nickname: "",
      password: "",
      confirmPassword: "",
    },
  });
  const mutation = useMutation({
    mutationFn: signUp,
    onError: (error) => form.setError("root", { message: errorMessage(error) }),
  });

  const submit = form.handleSubmit((values) => {
    mutation.mutate({
      username: values.username,
      email: values.email,
      password: values.password,
      nickname: values.nickname || undefined,
    });
  });

  return (
    <div>
      <div className="mb-7">
        <h2 className="text-3xl font-semibold">创建账户</h2>
        <p className="mt-2 text-sm leading-6 text-muted-foreground">建立你的社区身份。</p>
      </div>

      <form className="space-y-4" onSubmit={submit}>
        {form.formState.errors.root?.message ? (
          <Alert>{form.formState.errors.root.message}</Alert>
        ) : null}

        <Field id="username" label="用户名" error={form.formState.errors.username?.message}>
          <Input
            id="username"
            autoComplete="username"
            autoFocus
            aria-invalid={Boolean(form.formState.errors.username)}
            {...form.register("username")}
          />
        </Field>

        <Field id="email" label="邮箱" error={form.formState.errors.email?.message}>
          <Input
            id="email"
            type="email"
            autoComplete="email"
            aria-invalid={Boolean(form.formState.errors.email)}
            {...form.register("email")}
          />
        </Field>

        <Field id="nickname" label="昵称（可选）" error={form.formState.errors.nickname?.message}>
          <Input id="nickname" autoComplete="nickname" {...form.register("nickname")} />
        </Field>

        <Field id="register-password" label="密码" error={form.formState.errors.password?.message}>
          <div className="relative">
            <Input
              id="register-password"
              type={showPassword ? "text" : "password"}
              autoComplete="new-password"
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
        </Field>

        <Field
          id="confirm-password"
          label="确认密码"
          error={form.formState.errors.confirmPassword?.message}
        >
          <Input
            id="confirm-password"
            type={showPassword ? "text" : "password"}
            autoComplete="new-password"
            aria-invalid={Boolean(form.formState.errors.confirmPassword)}
            {...form.register("confirmPassword")}
          />
        </Field>

        <Button className="w-full gap-2" type="submit" disabled={mutation.isPending}>
          {mutation.isPending ? <LoadingIndicator /> : null}
          {mutation.isPending ? "正在创建" : "创建账户"}
        </Button>
      </form>

      <p className="mt-6 text-center text-sm text-muted-foreground">
        已有账户？{" "}
        <Link href="/login" className="font-medium text-primary hover:underline">
          返回登录
        </Link>
      </p>
    </div>
  );
}

function Field({
  id,
  label,
  error,
  children,
}: {
  id: string;
  label: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      {children}
      <p className="min-h-5 text-sm text-destructive">{error}</p>
    </div>
  );
}
