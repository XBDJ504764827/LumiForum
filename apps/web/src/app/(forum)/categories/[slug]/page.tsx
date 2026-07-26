import type { Metadata } from "next";

import { CategoryTopics } from "@/components/forum/category-topics";
import type { TopicSort } from "@lumiforum/types";

type Props = {
  params: Promise<{ slug: string }>;
  searchParams: Promise<{ sort?: string; page?: string }>;
};

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  return { title: `${decodeURIComponent(slug)} | LumiForum` };
}

export default async function CategoryPage({ params, searchParams }: Props) {
  const [{ slug }, query] = await Promise.all([params, searchParams]);
  const sort = validSort(query.sort) ? query.sort : "latest";
  const page = Math.max(1, Number.parseInt(query.page ?? "1", 10) || 1);
  return <CategoryTopics slug={decodeURIComponent(slug)} sort={sort} page={page} />;
}

function validSort(value: string | undefined): value is TopicSort {
  return value === "latest" || value === "hot" || value === "featured" || value === "pinned";
}
