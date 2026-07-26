import type { ReactNode } from "react";

import { ForumHeader } from "@/components/forum/forum-header";

export default function ForumLayout({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-white">
      <ForumHeader />
      {children}
      <footer className="mt-16 border-t border-border bg-surface">
        <div className="mx-auto flex min-h-20 max-w-6xl items-center justify-between gap-4 px-5 py-5 text-sm text-muted-foreground sm:px-8">
          <span>LumiForum</span>
          <span>Community forum</span>
        </div>
      </footer>
    </div>
  );
}
