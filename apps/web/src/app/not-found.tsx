import Link from "next/link";

import { getSiteName } from "@/lib/seo/site";

export default function NotFound() {
  const siteName = getSiteName();
  return (
    <main className="flex min-h-[60vh] flex-col items-center justify-center gap-4 px-5 text-center">
      <p className="text-5xl font-semibold text-muted-foreground/60">404</p>
      <h1 className="text-xl font-semibold">页面不存在</h1>
      <p className="max-w-md text-sm text-muted-foreground">
        你访问的内容可能已被删除、移动，或从未存在过。
      </p>
      <Link
        href="/"
        className="mt-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
      >
        返回{siteName}首页
      </Link>
    </main>
  );
}
