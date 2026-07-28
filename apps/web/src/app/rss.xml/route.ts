import { fetchTopics } from "@/lib/api/server";
import { buildTopicsRss, latestFeedTitle } from "@/lib/seo/rss";
import { getDefaultDescription } from "@/lib/seo/site";

export const revalidate = 300;

export async function GET() {
  const topics =
    (await fetchTopics({ page: 1, page_size: 30, sort: "latest" }, { revalidate: 300 }))?.items ??
    [];
  const body = buildTopicsRss({
    title: latestFeedTitle(),
    description: getDefaultDescription(),
    path: "/rss.xml",
    topics,
  });
  return new Response(body, {
    headers: {
      "Content-Type": "application/rss+xml; charset=utf-8",
      "Cache-Control": "public, s-maxage=300, stale-while-revalidate=600",
    },
  });
}
