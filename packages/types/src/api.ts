/** Shared API contracts — keep in sync with apps/api JSON responses. */

export interface HealthResponse {
  status: string;
  service: string;
  timestamp: string;
}

export interface ReadyResponse {
  status: string;
  postgres: string;
  redis: string;
  timestamp: string;
}

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
  };
}

export interface ApiResponse<T> {
  data: T;
}

export type UserStatus = "active" | "pending" | "suspended" | "disabled";

export interface RoleSummary {
  code: string;
  name: string;
}

export interface User {
  id: string;
  username: string;
  email: string;
  avatar: string | null;
  nickname: string | null;
  role: RoleSummary;
  status: UserStatus;
  email_verified: boolean;
  followers_count: number;
  following_count: number;
  created_at: string;
  updated_at: string;
}

export interface RegisterRequest {
  username: string;
  email: string;
  password: string;
  nickname?: string;
}

export interface LoginRequest {
  identifier: string;
  password: string;
}

export interface AuthResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
  user: User;
}

export interface TokenRefreshResponse {
  access_token: string;
  token_type: "Bearer";
  expires_in: number;
}

export interface ProfileUpdateRequest {
  nickname?: string | null;
}

export type UploadCategory = "avatar" | "topic_image" | "comment_image" | "attachment";

export interface Upload {
  id: string;
  user_id: string;
  filename: string;
  original_filename: string;
  storage_provider: "local" | "s3";
  mime_type: string;
  file_size: number;
  category: UploadCategory;
  url: string;
  thumbnail_url: string | null;
  width: number | null;
  height: number | null;
  created_at: string;
  updated_at: string;
}

export interface UploadListParams {
  category?: UploadCategory;
  page?: number;
  page_size?: number;
}

export interface CategorySummary {
  id: string;
  slug: string;
  name: string;
  icon: string | null;
}

export interface Category extends CategorySummary {
  description: string | null;
  sort_order: number;
  is_visible: boolean;
  topic_count: number;
  created_at: string;
  updated_at: string;
}

export interface TopicAuthor {
  id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  role: RoleSummary;
}

export interface TopicStats {
  views: number;
  replies: number;
  likes: number;
}

export interface TopicSummary {
  id: string;
  title: string;
  slug: string;
  summary: string | null;
  category: CategorySummary;
  author: TopicAuthor;
  stats: TopicStats;
  is_pinned: boolean;
  is_featured: boolean;
  last_reply_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface TopicDetail extends TopicSummary {
  content: string;
  liked_by_me: boolean;
  favorited_by_me: boolean;
  following_author: boolean;
}

export interface PaginationMeta {
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
}

export interface Paginated<T> {
  items: T[];
  pagination: PaginationMeta;
}

export type TopicSort = "latest" | "hot" | "featured" | "pinned";

export interface TopicListParams {
  category?: string;
  sort?: TopicSort;
  page?: number;
  page_size?: number;
}

export interface CreateTopicRequest {
  category_id: string;
  title: string;
  content: string;
  summary?: string;
}

export interface UpdateTopicRequest {
  category_id?: string;
  title?: string;
  content?: string;
  summary?: string | null;
}

export interface CommentStats {
  likes: number;
  replies: number;
}

export interface CommentNode {
  id: string;
  topic_id: string;
  parent_id: string | null;
  content: string;
  author: TopicAuthor;
  stats: CommentStats;
  edited_at: string | null;
  created_at: string;
  updated_at: string;
  liked_by_me: boolean;
  replies: CommentNode[];
}

export interface TopicLikeState {
  liked: boolean;
  like_count: number;
}

export interface CommentLikeState {
  liked: boolean;
  like_count: number;
}

export interface FavoriteState {
  favorited: boolean;
}

export interface FollowState {
  following: boolean;
  followers_count: number;
  following_count: number;
}

export interface FavoriteItem {
  favorited_at: string;
  topic: TopicSummary;
}

export interface UserPublicSummary {
  id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  role: RoleSummary;
  followers_count: number;
  following_count: number;
  is_following: boolean;
  created_at: string;
}

export interface ReactionListParams {
  page?: number;
  page_size?: number;
}

export interface CommentListParams {
  page?: number;
  page_size?: number;
}

export interface CreateCommentRequest {
  content: string;
}

export interface UpdateCommentRequest {
  content: string;
}

export type NotificationType =
  | "post_liked"
  | "comment_liked"
  | "comment_created"
  | "comment_replied"
  | "topic_favorited"
  | "user_followed"
  | "mentioned"
  | "system_message";

export type NotificationTargetType = "topic" | "comment" | "user" | "system";

export interface NotificationActor {
  id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  role: RoleSummary;
}

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  content: string;
  target_type: NotificationTargetType | null;
  target_id: string | null;
  metadata: Record<string, unknown>;
  is_read: boolean;
  actor: NotificationActor | null;
  created_at: string;
  stream_hint: string;
}

export interface UnreadCount {
  count: number;
}

export interface NotificationListParams {
  page?: number;
  page_size?: number;
  is_read?: boolean;
  type?: NotificationType;
}

export type SearchType = "topic" | "comment" | "user";
export type SearchSort = "relevance" | "latest" | "hot";

export interface SearchParams {
  q?: string;
  keyword?: string;
  type?: SearchType;
  category_id?: string;
  author_id?: string;
  sort?: SearchSort;
  page?: number;
  page_size?: number;
  limit?: number;
  from?: string;
  to?: string;
}

export interface SearchAuthor {
  id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  role: RoleSummary;
}

export interface TopicSearchHit {
  id: string;
  title: string;
  slug: string;
  summary: string | null;
  highlight: string;
  category: CategorySummary;
  author: SearchAuthor;
  stats: TopicStats;
  created_at: string;
  rank: number;
}

export interface CommentSearchHit {
  id: string;
  topic_id: string;
  topic_slug: string;
  topic_title: string;
  content_preview: string;
  highlight: string;
  author: SearchAuthor;
  like_count: number;
  created_at: string;
  rank: number;
}

export interface UserSearchHit {
  id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  role: RoleSummary;
  followers_count: number;
  following_count: number;
  highlight: string;
  created_at: string;
  rank: number;
}

export type SearchHit =
  | ({ kind: "topic" } & TopicSearchHit)
  | ({ kind: "comment" } & CommentSearchHit)
  | ({ kind: "user" } & UserSearchHit);

export interface SearchResponse {
  query: string;
  type: SearchType;
  sort: SearchSort;
  items: SearchHit[];
  pagination: PaginationMeta;
  engine: string;
}

export interface SearchSuggestionsResponse {
  query: string;
  suggestions: string[];
}

export interface HotKeyword {
  keyword: string;
  score: number;
}

export interface HotKeywordsResponse {
  keywords: HotKeyword[];
}
