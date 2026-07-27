import type { Metadata } from "next";

import { RequireAuth } from "@/components/auth/route-guards";
import { TopicEditor } from "@/components/forum/topic-editor";

export const metadata: Metadata = {
  title: "发布帖子 | LumiForum",
};

export default function NewTopicPage() {
  return (
    <RequireAuth>
      <TopicEditor mode="create" />
    </RequireAuth>
  );
}
