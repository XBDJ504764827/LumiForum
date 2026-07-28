import type { ReactNode } from "react";

import { AdminShell } from "@/components/admin/admin-shell";
import { RequireAdmin } from "@/components/auth/route-guards";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("管理后台", "LumiForum 管理后台，不对搜索引擎开放");

export default function AdminLayout({ children }: { children: ReactNode }) {
  return (
    <RequireAdmin>
      <AdminShell>{children}</AdminShell>
    </RequireAdmin>
  );
}
