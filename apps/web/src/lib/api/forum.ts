import type {
  Category,
  CommentListParams,
  CommentNode,
  CreateCommentRequest,
  CreateTopicRequest,
  Paginated,
  TopicDetail,
  TopicListParams,
  TopicSummary,
  UpdateCommentRequest,
  UpdateTopicRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const forumKeys = {
  categories: ["forum", "categories"] as const,
  category: (slug: string) => ["forum", "categories", slug] as const,
  topics: (params: TopicListParams) => ["forum", "topics", params] as const,
  topic: (slug: string) => ["forum", "topic", slug] as const,
  comments: (topicId: string, params: CommentListParams) =>
    ["forum", "comments", topicId, params] as const,
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

export function listComments(
  topicId: string,
  params: CommentListParams = {},
): Promise<Paginated<CommentNode>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<CommentNode>>(
    `/topics/${encodeURIComponent(topicId)}/comments${suffix}`,
  );
}

export function createComment(topicId: string, input: CreateCommentRequest): Promise<CommentNode> {
  return apiRequest<CommentNode>(
    `/topics/${encodeURIComponent(topicId)}/comments`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function replyToComment(
  commentId: string,
  input: CreateCommentRequest,
): Promise<CommentNode> {
  return apiRequest<CommentNode>(
    `/comments/${encodeURIComponent(commentId)}/reply`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function updateComment(
  commentId: string,
  input: UpdateCommentRequest,
): Promise<CommentNode> {
  return apiRequest<CommentNode>(
    `/comments/${encodeURIComponent(commentId)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export async function deleteComment(commentId: string): Promise<void> {
  await apiRequest<{ message: string }>(
    `/comments/${encodeURIComponent(commentId)}`,
    { method: "DELETE" },
    true,
  );
}

export function restoreComment(commentId: string): Promise<CommentNode> {
  return apiRequest<CommentNode>(
    `/comments/${encodeURIComponent(commentId)}/restore`,
    { method: "POST" },
    true,
  );
}
