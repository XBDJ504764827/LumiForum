import type { MetadataRoute } from "next";

import { fetchCategories, fetchTopics } from "@/lib/api/server";
import { absoluteUrl, categoryPath, topicPath } from "@/lib/seo/site";

const TOPIC_PAGE_SIZE = 100;
const MAX_SITEMAP_TOPIC_PAGES = 50;

export const revalidate = 3600;

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const now = new Date();
  const entries: MetadataRoute.Sitemap = [
    {
      url: absoluteUrl("/"),
      lastModified: now,
      changeFrequency: "hourly",
      priority: 1,
    },
    {
      url: absoluteUrl("/categories"),
      lastModified: now,
      changeFrequency: "daily",
      priority: 0.9,
    },
    {
      url: absoluteUrl("/search"),
      lastModified: now,
      changeFrequency: "weekly",
      priority: 0.3,
    },
    {
      url: absoluteUrl("/rss.xml"),
      lastModified: now,
      changeFrequency: "hourly",
      priority: 0.4,
    },
  ];

  const categories = (await fetchCategories({ revalidate: 3600 })) ?? [];
  for (const category of categories) {
    entries.push({
      url: absoluteUrl(categoryPath(category.slug)),
      lastModified: new Date(category.updated_at),
      changeFrequency: "daily",
      priority: 0.8,
    });
  }

  for (let page = 1; page <= MAX_SITEMAP_TOPIC_PAGES; page += 1) {
    const batch = await fetchTopics(
      { page, page_size: TOPIC_PAGE_SIZE, sort: "latest" },
      { revalidate: 3600 },
    );
    if (!batch || batch.items.length === 0) break;
    for (const topic of batch.items) {
      entries.push({
        url: absoluteUrl(topicPath(topic.slug)),
        lastModified: new Date(topic.updated_at),
        changeFrequency: "daily",
        priority: topic.is_pinned || topic.is_featured ? 0.8 : 0.6,
      });
    }
    if (page >= batch.pagination.total_pages) break;
  }

  return entries;
}
