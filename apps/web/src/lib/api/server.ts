import { getApiBaseUrl, joinUrl } from "@lumiforum/shared";
import type {
  ApiResponse,
  Category,
  Paginated,
  TopicDetail,
  TopicListParams,
  TopicSummary,
} from "@lumiforum/types";

const DEFAULT_REVALIDATE_SECONDS = 60;

type ServerFetchOptions = {
  revalidate?: number | false;
  tags?: string[];
};

async function serverApiRequest<T>(
  path: string,
  options: ServerFetchOptions = {},
): Promise<T | null> {
  const base = getApiBaseUrl({ isServer: true });
  const url = joinUrl(base, path);
  try {
    const response = await fetch(url, {
      headers: {
        accept: "application/json",
      },
      next: {
        revalidate:
          options.revalidate === false
            ? undefined
            : (options.revalidate ?? DEFAULT_REVALIDATE_SECONDS),
        tags: options.tags,
      },
      cache: options.revalidate === false ? "no-store" : undefined,
    });
    if (!response.ok) return null;
    const body = (await response.json()) as ApiResponse<T>;
    return body.data;
  } catch {
    return null;
  }
}

export function fetchCategories(options?: ServerFetchOptions): Promise<Category[] | null> {
  return serverApiRequest<Category[]>("/categories", {
    revalidate: 120,
    tags: ["categories"],
    ...options,
  });
}

export function fetchCategory(
  slug: string,
  options?: ServerFetchOptions,
): Promise<Category | null> {
  return serverApiRequest<Category>(`/categories/${encodeURIComponent(slug)}`, {
    revalidate: 120,
    tags: ["categories", `category:${slug}`],
    ...options,
  });
}

export function fetchTopics(
  params: TopicListParams = {},
  options?: ServerFetchOptions,
): Promise<Paginated<TopicSummary> | null> {
  const query = new URLSearchParams();
  if (params.category) query.set("category", params.category);
  if (params.sort) query.set("sort", params.sort);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return serverApiRequest<Paginated<TopicSummary>>(`/topics${suffix}`, {
    revalidate: 60,
    tags: ["topics"],
    ...options,
  });
}

export function fetchTopic(
  slug: string,
  options?: ServerFetchOptions,
): Promise<TopicDetail | null> {
  return serverApiRequest<TopicDetail>(`/topics/${encodeURIComponent(slug)}`, {
    revalidate: 30,
    tags: ["topics", `topic:${slug}`],
    ...options,
  });
}
