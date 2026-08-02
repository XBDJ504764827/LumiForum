import { RequireAuth } from "@/components/auth/route-guards";
import { NewTopicClient } from "@/components/forum/new-topic-client";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("发布帖子");

export default function NewTopicPage() {
  return (
    <RequireAuth>
      <NewTopicClient />
    </RequireAuth>
  );
}
