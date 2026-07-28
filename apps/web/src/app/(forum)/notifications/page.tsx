import { RequireAuth } from "@/components/auth/route-guards";
import { NotificationsView } from "@/components/forum/notifications-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("通知中心");

export default function NotificationsPage() {
  return (
    <RequireAuth>
      <NotificationsView />
    </RequireAuth>
  );
}
