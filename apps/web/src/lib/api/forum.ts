import type {
  Category,
  CommentLikeState,
  CommentListParams,
  CommentNode,
  CreateCommentRequest,
  CreateTopicRequest,
  FavoriteItem,
  FavoriteState,
  FollowState,
  Paginated,
  ReactionListParams,
  TopicDetail,
  TopicLikeState,
  TopicListParams,
  TopicSummary,
  UpdateCommentRequest,
  UpdateTopicRequest,
  UserPublicSummary,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";
import { sessionAccessToken } from "@/lib/auth/session";

export const forumKeys = {
  categories: ["forum", "categories"] as const,
  category: (slug: string) => ["forum", "categories", slug] as const,
  topics: (params: TopicListParams) => ["forum", "topics", params] as const,
  topic: (slug: string) => ["forum", "topic", slug] as const,
  comments: (topicId: string, params: CommentListParams) =>
    ["forum", "comments", topicId, params] as const,
  favorites: (params: ReactionListParams) => ["forum", "favorites", params] as const,
  followers: (userId: string, params: ReactionListParams) =>
    ["forum", "followers", userId, params] as const,
  following: (userId: string, params: ReactionListParams) =>
    ["forum", "following", userId, params] as const,
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

export async function getTopic(slug: string): Promise<TopicDetail> {
  return apiRequest<TopicDetail>(
    `/topics/${encodeURIComponent(slug)}`,
    { headers: await optionalAuthHeaders() },
    false,
  );
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

export async function listComments(
  topicId: string,
  params: CommentListParams = {},
): Promise<Paginated<CommentNode>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<CommentNode>>(
    `/topics/${encodeURIComponent(topicId)}/comments${suffix}`,
    { headers: await optionalAuthHeaders() },
    false,
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

export function likeTopic(topicId: string): Promise<TopicLikeState> {
  return apiRequest<TopicLikeState>(
    `/topics/${encodeURIComponent(topicId)}/like`,
    { method: "POST" },
    true,
  );
}

export function unlikeTopic(topicId: string): Promise<TopicLikeState> {
  return apiRequest<TopicLikeState>(
    `/topics/${encodeURIComponent(topicId)}/like`,
    { method: "DELETE" },
    true,
  );
}

export function likeComment(commentId: string): Promise<CommentLikeState> {
  return apiRequest<CommentLikeState>(
    `/comments/${encodeURIComponent(commentId)}/like`,
    { method: "POST" },
    true,
  );
}

export function unlikeComment(commentId: string): Promise<CommentLikeState> {
  return apiRequest<CommentLikeState>(
    `/comments/${encodeURIComponent(commentId)}/like`,
    { method: "DELETE" },
    true,
  );
}

export function favoriteTopic(topicId: string): Promise<FavoriteState> {
  return apiRequest<FavoriteState>(
    `/topics/${encodeURIComponent(topicId)}/favorite`,
    { method: "POST" },
    true,
  );
}

export function unfavoriteTopic(topicId: string): Promise<FavoriteState> {
  return apiRequest<FavoriteState>(
    `/topics/${encodeURIComponent(topicId)}/favorite`,
    { method: "DELETE" },
    true,
  );
}

export function listMyFavorites(
  params: ReactionListParams = {},
): Promise<Paginated<FavoriteItem>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<FavoriteItem>>(`/me/favorites${suffix}`, undefined, true);
}

export function followUser(userId: string): Promise<FollowState> {
  return apiRequest<FollowState>(
    `/users/${encodeURIComponent(userId)}/follow`,
    { method: "POST" },
    true,
  );
}

export function unfollowUser(userId: string): Promise<FollowState> {
  return apiRequest<FollowState>(
    `/users/${encodeURIComponent(userId)}/follow`,
    { method: "DELETE" },
    true,
  );
}

export async function listFollowers(
  userId: string,
  params: ReactionListParams = {},
): Promise<Paginated<UserPublicSummary>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<UserPublicSummary>>(
    `/users/${encodeURIComponent(userId)}/followers${suffix}`,
    { headers: await optionalAuthHeaders() },
    false,
  );
}

export async function listFollowing(
  userId: string,
  params: ReactionListParams = {},
): Promise<Paginated<UserPublicSummary>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<UserPublicSummary>>(
    `/users/${encodeURIComponent(userId)}/following${suffix}`,
    { headers: await optionalAuthHeaders() },
    false,
  );
}

async function optionalAuthHeaders(): Promise<HeadersInit> {
  try {
    const token = await sessionAccessToken();
    return { authorization: `Bearer ${token}` };
  } catch {
    return {};
  }
}
