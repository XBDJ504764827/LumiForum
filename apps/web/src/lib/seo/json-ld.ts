import type { Category, TopicDetail, TopicSummary } from "@lumiforum/types";

import {
  absoluteUrl,
  categoryPath,
  categoryUrl,
  getSiteName,
  getSiteUrl,
  topicPath,
  topicUrl,
} from "@/lib/seo/site";
import { plainText } from "@/lib/seo/utils";

export function websiteJsonLd() {
  const siteUrl = getSiteUrl();
  return {
    "@context": "https://schema.org",
    "@type": "WebSite",
    name: getSiteName(),
    url: siteUrl,
    potentialAction: {
      "@type": "SearchAction",
      target: `${siteUrl}/search?q={search_term_string}`,
      "query-input": "required name=search_term_string",
    },
  };
}

export function breadcrumbJsonLd(items: Array<{ name: string; path: string }>) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, index) => ({
      "@type": "ListItem",
      position: index + 1,
      name: item.name,
      item: absoluteUrl(item.path),
    })),
  };
}

export function categoryJsonLd(category: Category) {
  return {
    "@context": "https://schema.org",
    "@type": "CollectionPage",
    name: category.name,
    description: plainText(category.description || `${category.name} 板块`),
    url: categoryUrl(category.slug),
    isPartOf: {
      "@type": "WebSite",
      name: getSiteName(),
      url: getSiteUrl(),
    },
  };
}

export function topicJsonLd(topic: TopicDetail | (TopicSummary & { content?: string })) {
  const authorName = topic.author.nickname || topic.author.username;
  const content = "content" in topic ? topic.content : undefined;
  const description = plainText(topic.summary || content || topic.title);
  return {
    "@context": "https://schema.org",
    "@type": "DiscussionForumPosting",
    headline: topic.title,
    ...(content ? { articleBody: plainText(content, 5000) } : {}),
    description,
    url: topicUrl(topic.slug),
    mainEntityOfPage: topicUrl(topic.slug),
    datePublished: topic.created_at,
    dateModified: topic.updated_at,
    author: {
      "@type": "Person",
      name: authorName,
      url: absoluteUrl(`/users/${topic.author.id}/followers`),
    },
    interactionStatistic: [
      {
        "@type": "InteractionCounter",
        interactionType: "https://schema.org/LikeAction",
        userInteractionCount: topic.stats.likes,
      },
      {
        "@type": "InteractionCounter",
        interactionType: "https://schema.org/CommentAction",
        userInteractionCount: topic.stats.replies,
      },
      {
        "@type": "InteractionCounter",
        interactionType: "https://schema.org/ViewAction",
        userInteractionCount: topic.stats.views,
      },
    ],
    isPartOf: {
      "@type": "CollectionPage",
      name: topic.category.name,
      url: categoryUrl(topic.category.slug),
    },
  };
}

export function topicBreadcrumbs(topic: Pick<TopicDetail, "title" | "slug" | "category">) {
  return breadcrumbJsonLd([
    { name: "首页", path: "/" },
    { name: "板块", path: "/categories" },
    { name: topic.category.name, path: categoryPath(topic.category.slug) },
    { name: topic.title, path: topicPath(topic.slug) },
  ]);
}

export function categoryBreadcrumbs(category: Pick<Category, "name" | "slug">) {
  return breadcrumbJsonLd([
    { name: "首页", path: "/" },
    { name: "板块", path: "/categories" },
    { name: category.name, path: categoryPath(category.slug) },
  ]);
}
