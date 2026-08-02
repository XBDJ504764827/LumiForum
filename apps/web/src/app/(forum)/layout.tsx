import type { ReactNode } from "react";

import { ForumHeader } from "@/components/forum/forum-header";
import { getSiteName, getSiteUrl } from "@/lib/seo/site";

export default function ForumLayout({ children }: { children: ReactNode }) {
  const siteName = getSiteName();
  return (
    <div className="flex min-h-screen flex-col bg-white">
      <ForumHeader />
      <div className="flex-1">{children}</div>
      <footer className="mt-16 border-t border-border bg-surface">
        <div className="mx-auto flex min-h-20 max-w-6xl flex-col justify-between gap-3 px-5 py-5 text-sm text-muted-foreground sm:flex-row sm:items-center sm:px-8">
          <div className="flex flex-wrap items-center gap-4">
            <span>{siteName}</span>
            <a href="/rss.xml" className="hover:text-foreground">
              RSS
            </a>
            <a href="/sitemap.xml" className="hover:text-foreground">
              Sitemap
            </a>
          </div>
          <span>{getSiteUrl().replace(/^https?:\/\//, "")}</span>
        </div>
      </footer>
    </div>
  );
}
