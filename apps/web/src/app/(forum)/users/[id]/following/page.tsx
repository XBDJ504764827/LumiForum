import { FollowListView } from "@/components/forum/follow-list-view";
import { buildPageMetadata } from "@/lib/seo/metadata";

type Props = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: Props) {
  const { id } = await params;
  return buildPageMetadata({
    title: "关注",
    description: "用户关注列表",
    path: `/users/${encodeURIComponent(decodeURIComponent(id))}/following`,
    noIndex: true,
  });
}

export default async function FollowingPage({ params }: Props) {
  const { id } = await params;
  return <FollowListView userId={decodeURIComponent(id)} mode="following" />;
}
