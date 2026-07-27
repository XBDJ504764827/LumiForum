import type {
  AdminCommentItem,
  AdminCommentListParams,
  AdminDashboard,
  AdminFileItem,
  AdminFileListParams,
  AdminLogItem,
  AdminLogListParams,
  AdminTopicItem,
  AdminTopicListParams,
  AdminTopicUpdateRequest,
  AdminUserItem,
  AdminUserListParams,
  AdminUserUpdateRequest,
  Category,
  CreateCategoryRequest,
  CreateReportRequest,
  Paginated,
  ReportItem,
  ReportListParams,
  ResolveReportRequest,
  RoleOption,
  UpdateCategoryRequest,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const adminKeys = {
  all: ["admin"] as const,
  dashboard: ["admin", "dashboard"] as const,
  roles: ["admin", "roles"] as const,
  users: (params: AdminUserListParams) => ["admin", "users", params] as const,
  user: (id: string) => ["admin", "user", id] as const,
  topics: (params: AdminTopicListParams) => ["admin", "topics", params] as const,
  comments: (params: AdminCommentListParams) => ["admin", "comments", params] as const,
  categories: ["admin", "categories"] as const,
  files: (params: AdminFileListParams) => ["admin", "files", params] as const,
  reports: (params: ReportListParams) => ["admin", "reports", params] as const,
  logs: (params: AdminLogListParams) => ["admin", "logs", params] as const,
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
