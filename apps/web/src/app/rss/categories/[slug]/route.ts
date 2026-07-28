import { fetchCategory, fetchTopics } from "@/lib/api/server";
import { buildTopicsRss, categoryFeedDescription, categoryFeedTitle } from "@/lib/seo/rss";

export const revalidate = 300;

type Props = {
  params: Promise<{ slug: string }>;
};

export async function GET(_request: Request, { params }: Props) {
  const { slug: rawSlug } = await params;
  const slug = decodeURIComponent(rawSlug);
  const category = await fetchCategory(slug, { revalidate: 300 });
  if (!category) {
    return new Response("Not Found", { status: 404 });
  }
  const topics =
    (
      await fetchTopics(
        { category: slug, page: 1, page_size: 30, sort: "latest" },
        { revalidate: 300, tags: [`category:${slug}`, "topics"] },
      )
    )?.items ?? [];
  const body = buildTopicsRss({
    title: categoryFeedTitle(category),
    description: categoryFeedDescription(category),
    path: `/rss/categories/${encodeURIComponent(slug)}`,
    topics,
    category,
  });
  return new Response(body, {
    headers: {
      "Content-Type": "application/rss+xml; charset=utf-8",
      "Cache-Control": "public, s-maxage=300, stale-while-revalidate=600",
    },
  });
}
