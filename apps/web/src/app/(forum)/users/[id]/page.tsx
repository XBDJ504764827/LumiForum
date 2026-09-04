import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { UserProfileView } from "@/components/forum/user-profile-view";
import { fetchPublicUser, fetchTopics } from "@/lib/api/server";
import { getDefaultDescription } from "@/lib/seo/site";

type Props = { params: Promise<{ id: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { id: raw } = await params;
  const id = decodeURIComponent(raw);
  const user = await fetchPublicUser(id);
  if (!user) {
    return { title: "用户不存在", description: getDefaultDescription() };
  }
  const name = user.nickname || user.username;
  return {
    title: `${name} 的主页`,
    description: `${user.nickname || user.username} 在 LumiForum 发布的帖子与信息。`,
  };
}

export default async function UserProfilePage({ params }: Props) {
  const { id: raw } = await params;
  const id = decodeURIComponent(raw);
  const [user, topics] = await Promise.all([
    fetchPublicUser(id),
    fetchTopics({ author_id: id, sort: "latest", page: 1, page_size: 20 }),
  ]);
  if (!user) notFound();

  return <UserProfileView userId={id} initialUser={user} initialTopics={topics ?? undefined} />;
}
