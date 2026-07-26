import type { ReactNode } from "react";

import { Brand } from "@/components/brand";
import { GuestOnly } from "@/components/auth/route-guards";

export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-surface">
      <header className="border-b border-border bg-white">
        <div className="mx-auto flex h-16 max-w-6xl items-center px-5 sm:px-8">
          <Brand />
        </div>
      </header>
      <main className="mx-auto grid min-h-[calc(100vh-4rem)] max-w-6xl lg:grid-cols-[minmax(0,1fr)_420px]">
        <section className="hidden border-r border-border px-12 py-16 lg:flex lg:flex-col lg:justify-between">
          <div className="max-w-md">
            <p className="mb-5 text-sm font-medium text-primary">社区账户</p>
            <h1 className="text-5xl font-semibold leading-[1.08]">LumiForum</h1>
            <p className="mt-5 text-lg leading-8 text-muted-foreground">连接，从身份开始。</p>
          </div>
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <span className="h-px w-12 bg-accent" />
            安全认证中心
          </div>
        </section>
        <section className="flex items-center justify-center px-5 py-10 sm:px-8 lg:px-10">
          <div className="w-full max-w-sm">
            <GuestOnly>{children}</GuestOnly>
          </div>
        </section>
      </main>
    </div>
  );
}
