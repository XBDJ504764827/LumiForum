import { stripTrailingSlash } from "./utils";

const DEFAULT_SITE_URL = "http://localhost:3000";
const DEFAULT_SITE_NAME = "LumiForum";
const DEFAULT_DESCRIPTION = "现代化社区论坛 — 讨论、分享与协作";

export function getSiteUrl(): string {
  const raw =
    process.env.NEXT_PUBLIC_SITE_URL ??
    process.env.SITE_URL ??
    process.env.CORS_ORIGIN ??
    DEFAULT_SITE_URL;
  return stripTrailingSlash(raw);
}

export function getSiteName(): string {
  return process.env.NEXT_PUBLIC_SITE_NAME?.trim() || DEFAULT_SITE_NAME;
}

export function getDefaultDescription(): string {
  return process.env.NEXT_PUBLIC_SITE_DESCRIPTION?.trim() || DEFAULT_DESCRIPTION;
}

export function absoluteUrl(path = "/"): string {
  const base = getSiteUrl();
  if (!path || path === "/") return base;
  return `${base}${path.startsWith("/") ? path : `/${path}`}`;
}

export function topicPath(slug: string): string {
  return `/topics/${encodeURIComponent(slug)}`;
}

export function categoryPath(slug: string): string {
  return `/categories/${encodeURIComponent(slug)}`;
}

export function topicUrl(slug: string): string {
  return absoluteUrl(topicPath(slug));
}

export function categoryUrl(slug: string): string {
  return absoluteUrl(categoryPath(slug));
}
