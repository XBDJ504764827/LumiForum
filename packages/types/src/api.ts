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
  avatar?: string | null;
  nickname?: string | null;
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
  replies: CommentNode[];
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
