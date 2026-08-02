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
  steam_id: string | null;
  steam_persona_name: string | null;
  steam_avatar: string | null;
  steam_avatar_medium: string | null;
  steam_avatar_full: string | null;
  steam_profile_url: string | null;
  steam_country_code: string | null;
  has_password: boolean;
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

export interface SteamAuthorizationResponse {
  authorization_url: string;
}

export interface SteamUnbindRequest {
  password: string;
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

export type PollStatus = "active" | "closed";
export type PollType = "standard";

export interface PollOption {
  id: string;
  content: string;
  sort_order: number;
  vote_count: number;
}

export interface PollVoter {
  user_id: string;
  username: string;
  nickname: string | null;
  avatar: string | null;
  option_id: string;
}

export interface Poll {
  id: string;
  topic_id: string;
  topic_slug: string;
  topic_title: string;
  author_id: string;
  title: string;
  description: string | null;
  poll_type: PollType;
  status: PollStatus;
  multiple_choice: boolean;
  anonymous: boolean;
  allow_cancel: boolean;
  max_choices: number;
  expires_at: string | null;
  created_at: string;
  updated_at: string;
  options: PollOption[];
  total_votes: number;
  participant_count: number;
  my_votes: string[];
  can_vote: boolean;
  can_manage: boolean;
}

export interface PollResultOption {
  option_id: string;
  content: string;
  vote_count: number;
  percentage: number;
}

export interface PollResults {
  poll_id: string;
  topic_id: string;
  topic_slug: string;
  topic_title: string;
  title: string;
  status: PollStatus;
  multiple_choice: boolean;
  anonymous: boolean;
  expires_at: string | null;
  total_votes: number;
  participant_count: number;
  options: PollResultOption[];
  voters?: PollVoter[];
}

export interface HotPollItem {
  poll_id: string;
  topic_id: string;
  topic_slug: string;
  topic_title: string;
  poll_title: string;
  participant_count: number;
  option_count: number;
  is_closed: boolean;
  category: CategorySummary;
  created_at: string;
}

export interface CreatePollDraft {
  title: string;
  description?: string;
  multiple_choice?: boolean;
  anonymous?: boolean;
  allow_cancel?: boolean;
  max_choices?: number;
  expires_at?: string | null;
  options: string[];
}

export interface VotePollRequest {
  option_ids: string[];
}

export interface UpdatePollRequest {
  title?: string;
  description?: string | null;
  expires_at?: string | null;
  allow_cancel?: boolean;
  /** New options to append (edit mode). */
  options_to_add?: string[];
  /** Existing zero-vote options to remove (edit mode). */
  option_ids_to_remove?: string[];
}

export interface AdminPollItem {
  id: string;
  topic_id: string;
  topic_title: string;
  topic_slug: string;
  title: string;
  status: PollStatus;
  multiple_choice: boolean;
  anonymous: boolean;
  max_choices: number;
  option_count: number;
  participant_count: number;
  expires_at: string | null;
  created_at: string;
  updated_at: string;
  author_id: string;
  author_username: string;
}

export interface AdminPollListParams {
  q?: string;
  status?: PollStatus | "";
  page?: number;
  page_size?: number;
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
  has_poll: boolean;
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
  /** Restrict to topics authored by this user. */
  author_id?: string;
  sort?: TopicSort;
  page?: number;
  page_size?: number;
}

export interface CreateTopicRequest {
  category_id: string;
  title: string;
  content: string;
  summary?: string;
  poll?: CreatePollDraft;
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
  | "system_message"
  | "report_submitted"
  | "report_processed"
  | "content_hidden"
  | "content_deleted"
  | "topic_locked"
  | "user_warned"
  | "user_muted"
  | "user_banned"
  | "sanction_expiring"
  | "sanction_revoked"
  | "appeal_submitted"
  | "appeal_approved"
  | "appeal_rejected"
  | "moderation_inbox"
  | "poll_voted"
  | "poll_ended";

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
  has_poll?: boolean;
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
  has_poll: boolean;
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

export interface PresenceStatus {
  user_id: string;
  online: boolean;
  last_seen_at: string | null;
}

export interface DailyCount {
  date: string;
  count: number;
}

export interface HotTopicStat {
  id: string;
  title: string;
  slug: string;
  view_count: number;
  reply_count: number;
  like_count: number;
}

export interface AdminDashboard {
  users_total: number;
  topics_total: number;
  comments_total: number;
  uploads_total: number;
  reports_open: number;
  users_today: number;
  topics_today: number;
  active_users_7d: number;
  registrations_7d: DailyCount[];
  topics_7d: DailyCount[];
  hot_topics: HotTopicStat[];
}

export interface AdminUserItem {
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
  last_login_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface AdminUserListParams {
  q?: string;
  status?: UserStatus;
  role?: string;
  page?: number;
  page_size?: number;
}

export interface AdminUserUpdateRequest {
  status?: UserStatus;
  role?: string;
}

export interface RoleOption {
  code: string;
  name: string;
  priority: number;
}

export interface AdminTopicItem {
  id: string;
  title: string;
  slug: string;
  status: string;
  summary: string | null;
  category_id: string;
  category_name: string;
  category_slug: string;
  author_id: string;
  author_username: string;
  view_count: number;
  reply_count: number;
  like_count: number;
  is_pinned: boolean;
  is_featured: boolean;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface AdminTopicListParams {
  q?: string;
  status?: string;
  category_id?: string;
  page?: number;
  page_size?: number;
}

export interface AdminTopicUpdateRequest {
  status?: string;
  is_pinned?: boolean;
  is_featured?: boolean;
}

export interface AdminCommentItem {
  id: string;
  topic_id: string;
  topic_title: string;
  topic_slug: string;
  parent_id: string | null;
  content: string;
  status: string;
  author_id: string;
  author_username: string;
  like_count: number;
  reply_count: number;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface AdminCommentListParams {
  q?: string;
  status?: string;
  topic_id?: string;
  page?: number;
  page_size?: number;
}

export interface AdminFileItem {
  id: string;
  user_id: string;
  username: string;
  filename: string;
  original_filename: string;
  mime_type: string;
  file_size: number;
  category: UploadCategory;
  url: string | null;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface AdminFileListParams {
  q?: string;
  category?: UploadCategory;
  status?: string;
  page?: number;
  page_size?: number;
}

export type ReportTargetType = "topic" | "comment" | "user";
export type ReportStatus = "open" | "reviewing" | "resolved" | "rejected";

export interface CreateReportRequest {
  target_type: ReportTargetType;
  target_id: string;
  reason: string;
  details?: string;
}

export interface ResolveReportRequest {
  status: ReportStatus;
  resolution_note?: string;
}

export interface ReportItem {
  id: string;
  reporter_id: string;
  reporter_username: string;
  target_type: ReportTargetType;
  target_id: string;
  reason: string;
  details: string | null;
  status: ReportStatus;
  handler_id: string | null;
  handler_username: string | null;
  resolution_note: string | null;
  handled_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ReportListParams {
  status?: ReportStatus;
  target_type?: ReportTargetType;
  page?: number;
  page_size?: number;
}

export interface AdminLogItem {
  id: string;
  admin_id: string;
  admin_username: string;
  action: string;
  target_type: string;
  target_id: string | null;
  summary: string;
  metadata: Record<string, unknown>;
  ip_address: string | null;
  user_agent: string | null;
  created_at: string;
}

export interface AdminLogListParams {
  q?: string;
  action?: string;
  page?: number;
  page_size?: number;
}

export interface CreateCategoryRequest {
  slug?: string;
  name: string;
  description?: string;
  icon?: string;
  sort_order?: number;
  is_visible?: boolean;
}

export interface UpdateCategoryRequest {
  slug?: string;
  name?: string;
  description?: string | null;
  icon?: string | null;
  sort_order?: number;
  is_visible?: boolean;
}
