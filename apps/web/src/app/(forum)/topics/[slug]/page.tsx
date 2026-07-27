import type { Metadata } from "next";

import { TopicView } from "@/components/forum/topic-view";

type Props = { params: Promise<{ slug: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  return { title: `${decodeURIComponent(slug)} | LumiForum` };
}

export default async function TopicPage({ params }: Props) {
  const { slug } = await params;
  return <TopicView slug={decodeURIComponent(slug)} />;
}
