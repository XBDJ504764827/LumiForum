import { RequireAuth } from "@/components/auth/route-guards";
import { MessagesView } from "@/components/messages/messages-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("私信");

export default function MessagesPage() {
  return (
    <RequireAuth>
      <MessagesView />
    </RequireAuth>
  );
}
