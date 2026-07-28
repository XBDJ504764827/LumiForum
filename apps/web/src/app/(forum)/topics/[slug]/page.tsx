import type { Metadata } from "next";

import { TopicView } from "@/components/forum/topic-view";
import { JsonLd } from "@/components/seo/json-ld";
import { fetchTopic } from "@/lib/api/server";
import { topicBreadcrumbs, topicJsonLd } from "@/lib/seo/json-ld";
import { privatePageMetadata, topicMetadata } from "@/lib/seo/metadata";

type Props = { params: Promise<{ slug: string }> };

export const revalidate = 30;

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug: raw } = await params;
  const slug = decodeURIComponent(raw);
  const topic = await fetchTopic(slug);
  if (!topic) {
    return privatePageMetadata("帖子未找到", "该帖子不存在或已被删除");
  }
  return topicMetadata({
    title: topic.title,
    slug: topic.slug,
    summary: topic.summary,
    content: topic.content,
    categoryName: topic.category.name,
    authorName: topic.author.nickname || topic.author.username,
    image: topic.author.avatar,
    createdAt: topic.created_at,
    updatedAt: topic.updated_at,
  });
}

export default async function TopicPage({ params }: Props) {
  const { slug: raw } = await params;
  const slug = decodeURIComponent(raw);
  const topic = await fetchTopic(slug);

  return (
    <>
      {topic ? <JsonLd data={[topicJsonLd(topic), topicBreadcrumbs(topic)]} /> : null}
      <TopicView slug={slug} />
    </>
  );
}
