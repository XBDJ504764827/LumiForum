import type { Metadata } from "next";
import { notFound } from "next/navigation";
import type { TopicSort } from "@lumiforum/types";

import { CategoryTopics } from "@/components/forum/category-topics";
import { JsonLd } from "@/components/seo/json-ld";
import { fetchCategory, fetchTopics } from "@/lib/api/server";
import { categoryBreadcrumbs, categoryJsonLd } from "@/lib/seo/json-ld";
import { categoryMetadata, privatePageMetadata } from "@/lib/seo/metadata";

type Props = {
  params: Promise<{ slug: string }>;
  searchParams: Promise<{ sort?: string; page?: string }>;
};

export const revalidate = 60;

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug: raw } = await params;
  const slug = decodeURIComponent(raw);
  const category = await fetchCategory(slug);
  if (!category) {
    return privatePageMetadata("板块未找到", "该板块不存在或已隐藏");
  }
  return categoryMetadata(category);
}

export default async function CategoryPage({ params, searchParams }: Props) {
  const [{ slug: raw }, query] = await Promise.all([params, searchParams]);
  const slug = decodeURIComponent(raw);
  const sort = validSort(query.sort) ? query.sort : "latest";
  const page = Math.max(1, Number.parseInt(query.page ?? "1", 10) || 1);
  // Server-side fetch (ISR); hydrated into the client query cache via
  // initialData so the browser does not re-request the same data.
  const [category, topics] = await Promise.all([
    fetchCategory(slug),
    fetchTopics({ category: slug, sort, page, page_size: 20 }),
  ]);
  // A missing category must return a real 404 (not a 200 shell).
  if (!category) notFound();

  return (
    <>
      <JsonLd data={[categoryJsonLd(category), categoryBreadcrumbs(category)]} />
      <CategoryTopics
        slug={slug}
        sort={sort}
        page={page}
        initialCategory={category}
        initialTopics={topics ?? undefined}
      />
    </>
  );
}

function validSort(value: string | undefined): value is TopicSort {
  return value === "latest" || value === "hot" || value === "featured" || value === "pinned";
}
