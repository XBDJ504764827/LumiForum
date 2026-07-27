import type { Metadata } from "next";

import { FollowListView } from "@/components/forum/follow-list-view";

type Props = { params: Promise<{ id: string }> };

export const metadata: Metadata = {
  title: "粉丝 | LumiForum",
};

export default async function FollowersPage({ params }: Props) {
  const { id } = await params;
  return <FollowListView userId={decodeURIComponent(id)} mode="followers" />;
}
