import type { Metadata } from "next";

import {
  absoluteUrl,
  categoryPath,
  categoryUrl,
  getDefaultDescription,
  getSiteName,
  getSiteUrl,
  topicPath,
  topicUrl,
} from "@/lib/seo/site";
import { plainText, uniqueKeywords } from "@/lib/seo/utils";

type BuildPageMetadataInput = {
  title: string;
  description?: string | null;
  path?: string;
  keywords?: Array<string | null | undefined>;
  image?: string | null;
  type?: "website" | "article";
  noIndex?: boolean;
  publishedTime?: string | null;
  modifiedTime?: string | null;
  authors?: string[];
  section?: string | null;
};

export function rootMetadata(): Metadata {
  const siteName = getSiteName();
  const description = getDefaultDescription();
  const siteUrl = getSiteUrl();
  return {
    metadataBase: new URL(siteUrl),
    title: {
      default: siteName,
      template: `%s | ${siteName}`,
    },
    description,
    applicationName: siteName,
    keywords: [siteName, "论坛", "社区", "讨论"],
    authors: [{ name: siteName }],
    creator: siteName,
    publisher: siteName,
    alternates: {
      canonical: "/",
      types: {
        "application/rss+xml": [{ url: "/rss.xml", title: `${siteName} 最新帖子` }],
      },
    },
    openGraph: {
      type: "website",
      locale: "zh_CN",
      siteName,
      title: siteName,
      description,
      url: siteUrl,
    },
    twitter: {
      card: "summary_large_image",
      title: siteName,
      description,
    },
    robots: {
      index: true,
      follow: true,
      googleBot: {
        index: true,
        follow: true,
        "max-image-preview": "large",
        "max-snippet": -1,
        "max-video-preview": -1,
      },
    },
    formatDetection: {
      email: false,
      address: false,
      telephone: false,
    },
  };
}

export function buildPageMetadata(input: BuildPageMetadataInput): Metadata {
  const siteName = getSiteName();
  const description = plainText(input.description || getDefaultDescription());
  const path = input.path ?? "/";
  const canonical = absoluteUrl(path);
  const keywords = uniqueKeywords([siteName, ...(input.keywords ?? [])]);
  const images = input.image ? [{ url: input.image, alt: input.title }] : undefined;

  return {
    title: input.title,
    description,
    keywords,
    alternates: input.noIndex
      ? undefined
      : {
          canonical: path,
        },
    openGraph: input.noIndex
      ? {
          title: input.title,
          description,
        }
      : {
          type: input.type ?? "website",
          locale: "zh_CN",
          siteName,
          title: input.title,
          description,
          url: canonical,
          images,
          ...(input.type === "article"
            ? {
                publishedTime: input.publishedTime ?? undefined,
                modifiedTime: input.modifiedTime ?? undefined,
                authors: input.authors,
                section: input.section ?? undefined,
              }
            : {}),
        },
    twitter: {
      card: images ? "summary_large_image" : "summary",
      title: input.title,
      description,
      images: images?.map((image) => image.url),
    },
    robots: input.noIndex
      ? { index: false, follow: false, nocache: true }
      : { index: true, follow: true },
  };
}

export function homeMetadata(): Metadata {
  const siteName = getSiteName();
  return {
    ...buildPageMetadata({
      title: siteName,
      description: getDefaultDescription(),
      path: "/",
      keywords: ["首页", "最新讨论", "社区"],
    }),
    title: {
      absolute: siteName,
    },
  };
}

export function categoriesIndexMetadata(): Metadata {
  return buildPageMetadata({
    title: "板块",
    description: "浏览全部论坛板块与分类目录",
    path: "/categories",
    keywords: ["板块", "分类"],
  });
}

export function categoryMetadata(input: {
  name: string;
  slug: string;
  description?: string | null;
}): Metadata {
  return buildPageMetadata({
    title: input.name,
    description: input.description || `${input.name} 板块的最新讨论`,
    path: categoryPath(input.slug),
    keywords: [input.name, "板块", "分类"],
  });
}

export function topicMetadata(input: {
  title: string;
  slug: string;
  summary?: string | null;
  content?: string | null;
  categoryName?: string | null;
  authorName?: string | null;
  image?: string | null;
  createdAt?: string | null;
  updatedAt?: string | null;
}): Metadata {
  return buildPageMetadata({
    title: input.title,
    description: input.summary || input.content || `${input.title} — ${getSiteName()} 讨论`,
    path: topicPath(input.slug),
    keywords: [input.title, input.categoryName, input.authorName, "帖子"],
    image: input.image,
    type: "article",
    publishedTime: input.createdAt,
    modifiedTime: input.updatedAt,
    authors: input.authorName ? [input.authorName] : undefined,
    section: input.categoryName,
  });
}

export function privatePageMetadata(title: string, description?: string): Metadata {
  return buildPageMetadata({
    title,
    description: description || `${title} — 仅登录用户可访问`,
    noIndex: true,
  });
}

export function searchMetadata(query?: string): Metadata {
  if (query?.trim()) {
    return buildPageMetadata({
      title: `搜索：${query.trim()}`,
      description: `在 ${getSiteName()} 搜索「${query.trim()}」的结果`,
      path: `/search?q=${encodeURIComponent(query.trim())}`,
      noIndex: true,
      keywords: ["搜索", query.trim()],
    });
  }
  return buildPageMetadata({
    title: "搜索",
    description: `搜索 ${getSiteName()} 中的帖子、评论与用户`,
    path: "/search",
    keywords: ["搜索"],
  });
}

export { categoryUrl, topicUrl };
