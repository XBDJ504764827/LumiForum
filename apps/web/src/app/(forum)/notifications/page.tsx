import type { Metadata } from "next";

import { RequireAuth } from "@/components/auth/route-guards";
import { NotificationsView } from "@/components/forum/notifications-view";

export const metadata: Metadata = {
  title: "通知中心 | LumiForum",
};

export default function NotificationsPage() {
  return (
    <RequireAuth>
      <NotificationsView />
    </RequireAuth>
  );
}
