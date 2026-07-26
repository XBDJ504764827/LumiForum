import type {
  Category,
  CreateTopicRequest,
  Paginated,
  TopicDetail,
  TopicListParams,
  TopicSummary,
  UpdateTopicRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const forumKeys = {
  categories: ["forum", "categories"] as const,
  category: (slug: string) => ["forum", "categories", slug] as const,
  topics: (params: TopicListParams) => ["forum", "topics", params] as const,
  topic: (slug: string) => ["forum", "topic", slug] as const,
};

export function listCategories(): Promise<Category[]> {
  return apiRequest<Category[]>("/categories");
}

export function getCategory(slug: string): Promise<Category> {
  return apiRequest<Category>(`/categories/${encodeURIComponent(slug)}`);
}

export function listTopics(params: TopicListParams = {}): Promise<Paginated<TopicSummary>> {
  const query = new URLSearchParams();
  if (params.category) query.set("category", params.category);
  if (params.sort) query.set("sort", params.sort);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<TopicSummary>>(`/topics${suffix}`);
}

export function getTopic(slug: string): Promise<TopicDetail> {
  return apiRequest<TopicDetail>(`/topics/${encodeURIComponent(slug)}`);
}

export function createTopic(input: CreateTopicRequest): Promise<TopicDetail> {
  return apiRequest<TopicDetail>("/topics", { method: "POST", body: JSON.stringify(input) }, true);
}

export function updateTopic(topicId: string, input: UpdateTopicRequest): Promise<TopicDetail> {
  return apiRequest<TopicDetail>(
    `/topics/${encodeURIComponent(topicId)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export async function deleteTopic(topicId: string): Promise<void> {
  await apiRequest<{ message: string }>(
    `/topics/${encodeURIComponent(topicId)}`,
    { method: "DELETE" },
    true,
  );
}
