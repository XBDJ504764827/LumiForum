import { UserTopicsView } from "@/components/forum/user-topics-view";
import { buildPageMetadata } from "@/lib/seo/metadata";

type Props = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: Props) {
  const { id } = await params;
  return buildPageMetadata({
    title: "发布的帖子",
    description: "用户发布的帖子列表",
    path: `/users/${encodeURIComponent(decodeURIComponent(id))}/topics`,
    noIndex: true,
  });
}

export default async function UserTopicsPage({ params }: Props) {
  const { id } = await params;
  return <UserTopicsView userId={decodeURIComponent(id)} />;
}
