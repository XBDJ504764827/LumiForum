import type {
  HotKeywordsResponse,
  SearchParams,
  SearchResponse,
  SearchSuggestionsResponse,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const searchKeys = {
  all: ["search"] as const,
  results: (params: SearchParams) => ["search", "results", params] as const,
  suggestions: (q: string) => ["search", "suggestions", q] as const,
  hot: ["search", "hot"] as const,
};

export function search(params: SearchParams): Promise<SearchResponse> {
  const query = new URLSearchParams();
  const q = params.q ?? params.keyword;
  if (q) query.set("q", q);
  if (params.type) query.set("type", params.type);
  if (params.category_id) query.set("category_id", params.category_id);
  if (params.author_id) query.set("author_id", params.author_id);
  if (params.sort) query.set("sort", params.sort);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  if (params.limit) query.set("limit", String(params.limit));
  if (params.from) query.set("from", params.from);
  if (params.to) query.set("to", params.to);
  if (params.has_poll !== undefined) query.set("has_poll", String(params.has_poll));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<SearchResponse>(`/search${suffix}`);
}

export function searchSuggestions(q: string): Promise<SearchSuggestionsResponse> {
  const query = new URLSearchParams({ q });
  return apiRequest<SearchSuggestionsResponse>(`/search/suggestions?${query}`);
}

export function hotKeywords(): Promise<HotKeywordsResponse> {
  return apiRequest<HotKeywordsResponse>("/search/hot");
}

const RECENT_KEY = "lumiforum-recent-searches";
const RECENT_LIMIT = 8;

export function loadRecentSearches(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((item): item is string => typeof item === "string").slice(0, RECENT_LIMIT);
  } catch {
    return [];
  }
}

export function saveRecentSearch(keyword: string): string[] {
  if (typeof window === "undefined") return [];
  const normalized = keyword.trim();
  if (!normalized) return loadRecentSearches();
  const next = [normalized, ...loadRecentSearches().filter((item) => item !== normalized)].slice(
    0,
    RECENT_LIMIT,
  );
  window.localStorage.setItem(RECENT_KEY, JSON.stringify(next));
  return next;
}

export function clearRecentSearches(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(RECENT_KEY);
}
