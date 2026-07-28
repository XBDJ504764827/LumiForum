import { RequireAuth } from "@/components/auth/route-guards";
import { TopicEditView } from "@/components/forum/topic-edit-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

type Props = { params: Promise<{ slug: string }> };

export const metadata = privatePageMetadata("编辑帖子");

export default async function EditTopicPage({ params }: Props) {
  const { slug } = await params;
  return (
    <RequireAuth>
      <TopicEditView slug={decodeURIComponent(slug)} />
    </RequireAuth>
  );
}
