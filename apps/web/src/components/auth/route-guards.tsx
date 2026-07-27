"use client";

import { useRouter } from "next/navigation";
import { useEffect, type ReactNode } from "react";

import { LoadingIndicator } from "@/components/loading-indicator";

import { useAuth } from "./auth-provider";

export function RequireAuth({ children }: { children: ReactNode }) {
  const router = useRouter();
  const { status } = useAuth();

  useEffect(() => {
    if (status === "unauthenticated") {
      router.replace("/login");
    }
  }, [router, status]);

  if (status !== "authenticated") {
    return <GuardLoading />;
  }
  return children;
}

export function GuestOnly({ children }: { children: ReactNode }) {
  const router = useRouter();
  const { status } = useAuth();

  useEffect(() => {
    if (status === "authenticated") {
      router.replace("/");
    }
  }, [router, status]);

  if (status !== "unauthenticated") {
    return <GuardLoading />;
  }
  return children;
}

export function RequireAdmin({ children }: { children: ReactNode }) {
  const router = useRouter();
  const { status, user } = useAuth();
  const allowed = user?.role.code === "administrator" || user?.role.code === "super_administrator";

  useEffect(() => {
    if (status === "unauthenticated") {
      router.replace("/login");
      return;
    }
    if (status === "authenticated" && !allowed) {
      router.replace("/");
    }
  }, [allowed, router, status]);

  if (status !== "authenticated" || !allowed) {
    return <GuardLoading />;
  }
  return children;
}

function GuardLoading() {
  return (
    <div className="flex min-h-48 items-center justify-center text-sm text-muted-foreground">
      <LoadingIndicator className="mr-2" />
      正在确认登录状态
    </div>
  );
}
