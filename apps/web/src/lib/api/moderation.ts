import type {
  CaseItem,
  CaseListParams,
  ModerationStatus,
  Paginated,
  PendingReviewItem,
  ReportItemV2,
  ReportListParams,
  ResolveReportRequestV2,
  RuleItem,
  RuleListParams,
  RuleRequest,
  SanctionItem,
  SanctionListParams,
} from "@lumiforum/types";

import { apiRequest } from "@/lib/api/client";

export const moderationKeys = {
  all: ["moderation"] as const,
  myStatus: ["moderation", "me"] as const,
  reports: (params: ReportListParams) => ["moderation", "reports", params] as const,
  reviews: (params: { target_type?: string; page?: number; page_size?: number }) =>
    ["moderation", "reviews", params] as const,
  cases: (params: { status?: string; page?: number; page_size?: number }) =>
    ["moderation", "cases", params] as const,
  sanctions: (params: SanctionListParams) => ["moderation", "sanctions", params] as const,
  rules: (params: RuleListParams) => ["moderation", "rules", params] as const,
  myReports: (params: { page?: number; page_size?: number }) =>
    ["moderation", "my-reports", params] as const,
};

// --- User-facing -----------------------------------------------------------

export function getMyModerationStatus(): Promise<ModerationStatus> {
  return apiRequest<ModerationStatus>("/moderation/me", {}, true);
}

export function createReport(input: {
  target_type: "topic" | "comment" | "user" | "file";
  target_id: string;
  reason_code?: string;
  reason?: string;
  details?: string;
}): Promise<ReportItemV2> {
  return apiRequest<ReportItemV2>(
    "/reports",
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function listMyReports(
  params: { page?: number; page_size?: number } = {},
): Promise<Paginated<ReportItemV2>> {
  const query = new URLSearchParams();
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<ReportItemV2>>(`/reports/me${suffix}`, {}, true);
}

// --- Admin moderation center ----------------------------------------------

export function listModerationReports(
  params: ReportListParams = {},
): Promise<Paginated<ReportItemV2>> {
  const query = new URLSearchParams();
  if (params.status) query.set("status", params.status);
  if (params.target_type) query.set("target_type", params.target_type);
  if (params.reason) query.set("reason", params.reason);
  if (params.priority) query.set("priority", params.priority);
  if (params.q) query.set("q", params.q);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<ReportItemV2>>(`/admin/moderation/reports${suffix}`, {}, true);
}

export function resolveModerationReport(
  id: string,
  input: ResolveReportRequestV2,
): Promise<ReportItemV2> {
  return apiRequest<ReportItemV2>(
    `/admin/moderation/reports/${encodeURIComponent(id)}/resolve`,
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function listPendingReviews(
  params: {
    target_type?: string;
    page?: number;
    page_size?: number;
  } = {},
): Promise<Paginated<PendingReviewItem>> {
  const query = new URLSearchParams();
  if (params.target_type) query.set("target_type", params.target_type);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<PendingReviewItem>>(`/admin/moderation/reviews${suffix}`, {}, true);
}

export function reviewContent(
  targetType: "topic" | "comment",
  id: string,
  approve: boolean,
  note?: string,
): Promise<ModerationStatus> {
  return apiRequest<ModerationStatus>(
    `/admin/moderation/reviews/${targetType}/${encodeURIComponent(id)}/${approve ? "approve" : "reject"}`,
    { method: "POST", body: JSON.stringify({ note: note ?? undefined }) },
    true,
  );
}

export function listModerationCases(params: CaseListParams = {}): Promise<Paginated<CaseItem>> {
  const query = new URLSearchParams();
  if (params.status) query.set("status", params.status);
  if (params.priority) query.set("priority", params.priority);
  if (params.source) query.set("source", params.source);
  if (params.target_type) query.set("target_type", params.target_type);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<CaseItem>>(`/admin/moderation/cases${suffix}`, {}, true);
}

export function listModerationSanctions(
  params: SanctionListParams = {},
): Promise<Paginated<SanctionItem>> {
  const query = new URLSearchParams();
  if (params.status) query.set("status", params.status);
  if (params.sanction_type) query.set("sanction_type", params.sanction_type);
  if (params.user_id) query.set("user_id", params.user_id);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<SanctionItem>>(`/admin/moderation/sanctions${suffix}`, {}, true);
}

export function listModerationRules(params: RuleListParams = {}): Promise<Paginated<RuleItem>> {
  const query = new URLSearchParams();
  if (params.status) query.set("status", params.status);
  if (params.rule_type) query.set("rule_type", params.rule_type);
  if (params.page) query.set("page", String(params.page));
  if (params.page_size) query.set("page_size", String(params.page_size));
  const suffix = query.size ? `?${query}` : "";
  return apiRequest<Paginated<RuleItem>>(`/admin/moderation/rules${suffix}`, {}, true);
}

export function createModerationRule(input: RuleRequest): Promise<RuleItem> {
  return apiRequest<RuleItem>(
    "/admin/moderation/rules",
    { method: "POST", body: JSON.stringify(input) },
    true,
  );
}

export function updateModerationRule(id: string, input: Partial<RuleRequest>): Promise<RuleItem> {
  return apiRequest<RuleItem>(
    `/admin/moderation/rules/${encodeURIComponent(id)}`,
    { method: "PATCH", body: JSON.stringify(input) },
    true,
  );
}

export function deleteModerationRule(id: string): Promise<{ message: string }> {
  return apiRequest<{ message: string }>(
    `/admin/moderation/rules/${encodeURIComponent(id)}`,
    { method: "DELETE" },
    true,
  );
}
