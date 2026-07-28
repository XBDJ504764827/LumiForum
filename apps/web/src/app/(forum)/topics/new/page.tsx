import { RequireAuth } from "@/components/auth/route-guards";
import { TopicEditor } from "@/components/forum/topic-editor";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("发布帖子");

export default function NewTopicPage() {
  return (
    <RequireAuth>
      <TopicEditor mode="create" />
    </RequireAuth>
  );
}
