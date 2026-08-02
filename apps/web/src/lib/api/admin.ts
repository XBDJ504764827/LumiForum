import type {
  AdminAnalytics,
  AdminCommentItem,
  AdminCommentListParams,
  AdminDashboard,
  AdminFileItem,
  AdminFileListParams,
  AdminLogItem,
  AdminLogListParams,
  AdminUserDetail,
  AdminPollItem,
  AdminPollListParams,
  AdminDashboardRange,
  AdminTopicItem,
  AdminTopicListParams,
  AdminTopicUpdateRequest,
  AdminUserItem,
  AdminUserListParams,
  AdminUserUpdateRequest,
  Category,
  CreateCategoryRequest,
  CreateReportRequest,
  LoginRecordItem,
  Paginated,
  PermissionOption,
  QueueSummary,
  ReportItem,
  ReportListParams,
  ResolveReportRequest,
  RoleOption,
  RolePermissionView,
  SystemSettingItem,
  UpdateCategoryRequest,
  UpdateRolePermissionsRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const adminKeys = {
  all: ["admin"] as const,
  dashboard: ["admin", "dashboard"] as const,
  dashboardRange: (range: AdminDashboardRange) => ["admin", "dashboard", range] as const,
  roles: ["admin", "roles"] as const,
  users: (params: AdminUserListParams) => ["admin", "users", params] as const,
  user: (id: string) => ["admin", "user", id] as const,
  topics: (params: AdminTopicListParams) => ["admin", "topics", params] as const,
  comments: (params: AdminCommentListParams) => ["admin", "comments", params] as const,
  categories: ["admin", "categories"] as const,
  files: (params: AdminFileListParams) => ["admin", "files", params] as const,
  reports: (params: ReportListParams) => ["admin", "reports", params] as const,
  polls: (params: AdminPollListParams) => ["admin", "polls", params] as const,
  logs: (params: AdminLogListParams) => ["admin", "logs", params] as const,
  queue: ["admin", "queue"] as const,
  analytics: (days: number) => ["admin", "analytics", days] as const,
  settings: ["admin", "settings"] as const,
  permissions: ["admin", "permissions"] as const,
  rolePermissions: (code: string) => ["admin", "role-permissions", code] as const,
  userDetail: (id: string) => ["admin", "user-detail", id] as const,
  loginRecords: (id: string, params: { page?: number; page_size?: number }) =>
    ["admin", "login-records", id, params] as const,
};

function queryString(params: Record<string, string | number | undefined>): string {
  const query = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== "") query.set(key, String(value));
  }
  const suffix = query.size ? `?${query}` : "";
  return suffix;
}

export function getAdminDashboard(): Promise<AdminDashboard> {
  return apiRequest<AdminDashboard>("/admin/dashboard", {}, true);
}

export function listAdminRoles(): Promise<RoleOption[]> {
  return apiRequest<RoleOption[]>("/admin/roles", {}, true);
}

export function listAdminUsers(
  params: AdminUserListParams = {},
): Promise<Paginated<AdminUserItem>> {
  return apiRequest(
    `/admin/users${queryString({
      q: params.q,
      status: params.status,
      role: params.role,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export function getAdminUser(id: string): Promise<AdminUserItem> {
  return apiRequest(`/admin/users/${encodeURIComponent(id)}`, {}, true);
}

export function updateAdminUser(id: string, input: AdminUserUpdateRequest): Promise<AdminUserItem> {
  return apiRequest(
    `/admin/users/${encodeURIComponent(id)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export function deleteAdminUser(id: string): Promise<AdminUserItem> {
  return apiRequest(`/admin/users/${encodeURIComponent(id)}`, { method: "DELETE" }, true);
}

export function listAdminTopics(
  params: AdminTopicListParams = {},
): Promise<Paginated<AdminTopicItem>> {
  return apiRequest(
    `/admin/topics${queryString({
      q: params.q,
      status: params.status,
      category_id: params.category_id,
      sort: params.sort || undefined,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export function updateAdminTopic(
  id: string,
  input: AdminTopicUpdateRequest,
): Promise<AdminTopicItem> {
  return apiRequest(
    `/admin/topics/${encodeURIComponent(id)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export async function deleteAdminTopic(id: string): Promise<void> {
  await apiRequest(`/admin/topics/${encodeURIComponent(id)}`, { method: "DELETE" }, true);
}

export function listAdminComments(
  params: AdminCommentListParams = {},
): Promise<Paginated<AdminCommentItem>> {
  return apiRequest(
    `/admin/comments${queryString({
      q: params.q,
      status: params.status,
      topic_id: params.topic_id,
      filter: params.filter || undefined,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export async function deleteAdminComment(id: string): Promise<void> {
  await apiRequest(`/admin/comments/${encodeURIComponent(id)}`, { method: "DELETE" }, true);
}

export function restoreAdminComment(id: string): Promise<AdminCommentItem> {
  return apiRequest(`/admin/comments/${encodeURIComponent(id)}/restore`, { method: "POST" }, true);
}

export function listAdminCategories(): Promise<Category[]> {
  return apiRequest("/admin/categories", {}, true);
}

export function createAdminCategory(input: CreateCategoryRequest): Promise<Category> {
  return apiRequest("/admin/categories", { method: "POST", body: JSON.stringify(input) }, true);
}

export function updateAdminCategory(id: string, input: UpdateCategoryRequest): Promise<Category> {
  return apiRequest(
    `/admin/categories/${encodeURIComponent(id)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export async function deleteAdminCategory(id: string): Promise<void> {
  await apiRequest(`/admin/categories/${encodeURIComponent(id)}`, { method: "DELETE" }, true);
}

export function listAdminFiles(
  params: AdminFileListParams = {},
): Promise<Paginated<AdminFileItem>> {
  return apiRequest(
    `/admin/files${queryString({
      q: params.q,
      category: params.category,
      status: params.status,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export async function deleteAdminFile(id: string): Promise<void> {
  await apiRequest(`/admin/files/${encodeURIComponent(id)}`, { method: "DELETE" }, true);
}

export function cleanupAdminFiles(): Promise<{ cleaned: number }> {
  return apiRequest("/admin/files/cleanup", { method: "POST" }, true);
}

export function listAdminReports(params: ReportListParams = {}): Promise<Paginated<ReportItem>> {
  return apiRequest(
    `/admin/reports${queryString({
      status: params.status,
      target_type: params.target_type,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export function resolveAdminReport(id: string, input: ResolveReportRequest): Promise<ReportItem> {
  return apiRequest(
    `/admin/reports/${encodeURIComponent(id)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export function createReport(input: CreateReportRequest): Promise<ReportItem> {
  return apiRequest("/reports", { method: "POST", body: JSON.stringify(input) }, true);
}

export function listAdminLogs(params: AdminLogListParams = {}): Promise<Paginated<AdminLogItem>> {
  return apiRequest(
    `/admin/logs${queryString({
      q: params.q,
      action: params.action,
      target_type: params.target_type,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export function getAdminUserDetail(id: string): Promise<AdminUserDetail> {
  return apiRequest(`/admin/users/${encodeURIComponent(id)}/detail`, {}, true);
}

export function listAdminLoginRecords(
  id: string,
  params: { page?: number; page_size?: number } = {},
): Promise<Paginated<LoginRecordItem>> {
  return apiRequest(
    `/admin/users/${encodeURIComponent(id)}/login-records${queryString(params)}`,
    {},
    true,
  );
}

export function forceAdminLogout(id: string): Promise<{ message: string }> {
  return apiRequest(
    `/admin/users/${encodeURIComponent(id)}/force-logout`,
    { method: "POST" },
    true,
  );
}

export function getAdminDashboardRange(range: AdminDashboardRange): Promise<AdminDashboard> {
  return apiRequest(`/admin/dashboard?range=${range}`, {}, true);
}

export function listAdminPermissions(): Promise<PermissionOption[]> {
  return apiRequest("/admin/permissions", {}, true);
}

export function getAdminRolePermissions(code: string): Promise<RolePermissionView> {
  return apiRequest(`/admin/roles/${encodeURIComponent(code)}/permissions`, {}, true);
}

export function updateAdminRolePermissions(
  code: string,
  input: UpdateRolePermissionsRequest,
): Promise<RolePermissionView> {
  return apiRequest(
    `/admin/roles/${encodeURIComponent(code)}/permissions`,
    { method: "PUT", body: JSON.stringify(input) },
    true,
  );
}

export function getAdminQueue(): Promise<QueueSummary> {
  return apiRequest("/admin/queue", {}, true);
}

export function getAdminAnalytics(days: number): Promise<AdminAnalytics> {
  return apiRequest(`/admin/analytics?days=${days}`, {}, true);
}

export function getAdminSettings(): Promise<SystemSettingItem[]> {
  return apiRequest("/admin/settings", {}, true);
}

export function updateAdminSettings(
  settings: Array<{ key: string; value: string | number | boolean }>,
): Promise<SystemSettingItem[]> {
  return apiRequest("/admin/settings", { method: "PUT", body: JSON.stringify({ settings }) }, true);
}

export function listAdminPolls(
  params: AdminPollListParams = {},
): Promise<Paginated<AdminPollItem>> {
  return apiRequest(
    `/admin/polls${queryString({
      q: params.q,
      status: params.status || undefined,
      page: params.page,
      page_size: params.page_size,
    })}`,
    {},
    true,
  );
}

export function isAdminRole(roleCode: string | undefined): boolean {
  return roleCode === "administrator" || roleCode === "super_administrator";
}
