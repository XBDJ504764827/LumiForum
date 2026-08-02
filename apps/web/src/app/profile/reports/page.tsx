import { RequireAuth } from "@/components/auth/route-guards";
import { MyReportsView } from "@/components/profile/my-reports-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("我的举报");

export default function MyReportsPage() {
  return (
    <RequireAuth>
      <MyReportsView />
    </RequireAuth>
  );
}
