import type { Category, TopicSummary } from "@lumiforum/types";

import { absoluteUrl, categoryUrl, getSiteName, getSiteUrl, topicUrl } from "@/lib/seo/site";
import { plainText, rfc822, xmlEscape } from "@/lib/seo/utils";

export function buildTopicsRss(input: {
  title: string;
  description: string;
  path: string;
  topics: TopicSummary[];
  category?: Category | null;
}): string {
  const channelLink = absoluteUrl(input.path.replace(/\.xml$/, "").replace(/\/rss$/, "") || "/");
  const selfLink = absoluteUrl(input.path);
  const items = input.topics
    .map((topic) => {
      const link = topicUrl(topic.slug);
      const description = plainText(topic.summary || topic.title, 300);
      const author = topic.author.nickname || topic.author.username;
      return `    <item>
      <title>${xmlEscape(topic.title)}</title>
      <link>${xmlEscape(link)}</link>
      <guid isPermaLink="true">${xmlEscape(link)}</guid>
      <pubDate>${rfc822(topic.created_at)}</pubDate>
      <description>${xmlEscape(description)}</description>
      <author>${xmlEscape(author)}</author>
      <category>${xmlEscape(topic.category.name)}</category>
    </item>`;
    })
    .join("\n");

  return `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${xmlEscape(input.title)}</title>
    <link>${xmlEscape(channelLink)}</link>
    <description>${xmlEscape(input.description)}</description>
    <language>zh-CN</language>
    <lastBuildDate>${rfc822(new Date())}</lastBuildDate>
    <generator>${xmlEscape(getSiteName())}</generator>
    <atom:link href="${xmlEscape(selfLink)}" rel="self" type="application/rss+xml" />
${input.category ? `    <category>${xmlEscape(input.category.name)}</category>\n` : ""}${items}
  </channel>
</rss>
`;
}

export function latestFeedTitle(): string {
  return `${getSiteName()} 最新帖子`;
}

export function categoryFeedTitle(category: Category): string {
  return `${category.name} — ${getSiteName()}`;
}

export function categoryFeedDescription(category: Category): string {
  return plainText(category.description || `${category.name} 板块最新讨论`);
}

export function siteHome(): string {
  return getSiteUrl();
}

export function categoryHome(slug: string): string {
  return categoryUrl(slug);
}
