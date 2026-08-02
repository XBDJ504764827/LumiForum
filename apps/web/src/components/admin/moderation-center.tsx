"use client";

import type {
  ModerationReportStatus,
  ModerationTargetType,
  ReportListParams,
  ReportItemV2,
  RuleItem,
  RuleRequest,
  SanctionItem,
  SanctionType,
} from "@lumiforum/types";
import { Button, Input, Select } from "@lumiforum/ui";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";

import {
  AdminPageHeader,
  AdminPagination,
  AdminTable,
  AdminToolbar,
  formatDateTime,
} from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { errorMessage } from "@/lib/api/errors";
import {
  createModerationRule,
  listModerationCases,
  listModerationReports,
  listModerationRules,
  listModerationSanctions,
  listPendingReviews,
  moderationKeys,
  resolveModerationReport,
  reviewContent,
  updateModerationRule,
} from "@/lib/api/moderation";

type Tab = "reviews" | "reports" | "cases" | "sanctions" | "rules";

const tabs: Array<{ value: Tab; label: string }> = [
  { value: "reviews", label: "待审内容" },
  { value: "reports", label: "举报" },
  { value: "cases", label: "审核队列" },
  { value: "sanctions", label: "处罚记录" },
  { value: "rules", label: "敏感词规则" },
];

export function AdminModerationView() {
  const [tab, setTab] = useState<Tab>("reviews");
  return (
    <div>
      <AdminPageHeader title="治理中心" description="举报处理、内容审核、用户处罚与规则管理。" />
      <div className="mb-5 flex flex-wrap gap-1 border-b border-border" role="tablist">
        {tabs.map((item) => (
          <button
            key={item.value}
            type="button"
            role="tab"
            aria-selected={tab === item.value}
            onClick={() => setTab(item.value)}
            className={`border-b-2 px-3 py-2 text-sm font-medium ${
              tab === item.value
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {item.label}
          </button>
        ))}
      </div>
      {tab === "reviews" ? <ReviewsTab /> : null}
      {tab === "reports" ? <ReportsTab /> : null}
      {tab === "cases" ? <CasesTab /> : null}
      {tab === "sanctions" ? <SanctionsTab /> : null}
      {tab === "rules" ? <RulesTab /> : null}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Pending reviews
// ---------------------------------------------------------------------------

function ReviewsTab() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState({ page: 1, page_size: 20 });
  const [error, setError] = useState<string | null>(null);
  const reviews = useQuery({
    queryKey: moderationKeys.reviews(params),
    queryFn: () => listPendingReviews(params),
  });
  const mutation = useMutation({
    mutationFn: ({ id, approve }: { id: string; approve: boolean }) => {
      const item = reviews.data?.items.find((item) => item.id === id);
      return reviewContent(item?.target_type ?? "topic", id, approve);
    },
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: moderationKeys.reviews(params) });
      await queryClient.invalidateQueries({ queryKey: moderationKeys.all });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (reviews.isPending) return <QueryLoading label="正在加载待审内容" />;
  if (reviews.isError) return <QueryError message="待审内容加载失败" />;

  return (
    <div>
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      {reviews.data.items.length === 0 ? (
        <p className="border-y border-border py-12 text-center text-sm text-muted-foreground">
          队列为空，暂无待审核内容
        </p>
      ) : (
        <AdminTable headers={["内容", "作者", "类型", "风险", "时间", "操作"]}>
          {reviews.data.items.map((item) => (
            <tr key={`${item.target_type}-${item.id}`}>
              <td className="max-w-md px-3 py-3">
                <div className="font-medium">{item.title}</div>
                <div className="line-clamp-2 text-sm text-muted-foreground">{item.snippet}</div>
              </td>
              <td className="px-3 py-3">@{item.author_username}</td>
              <td className="px-3 py-3">{item.target_type === "topic" ? "帖子" : "评论"}</td>
              <td className="px-3 py-3">
                <span
                  className={`rounded-sm px-1.5 py-0.5 text-xs ${
                    item.risk_score >= 80
                      ? "bg-destructive/10 text-destructive"
                      : item.risk_score >= 60
                        ? "bg-amber-500/10 text-amber-600"
                        : "bg-muted text-muted-foreground"
                  }`}
                >
                  {item.risk_score}
                </span>
              </td>
              <td className="px-3 py-3">{formatDateTime(item.created_at)}</td>
              <td className="px-3 py-3">
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    disabled={mutation.isPending}
                    onClick={() => mutation.mutate({ id: item.id, approve: true })}
                  >
                    通过
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    className="text-destructive hover:text-destructive"
                    disabled={mutation.isPending}
                    onClick={() => {
                      if (window.confirm("拒绝后内容将被隐藏，作者违规积分 +10。确认？"))
                        mutation.mutate({ id: item.id, approve: false });
                    }}
                  >
                    拒绝
                  </Button>
                </div>
              </td>
            </tr>
          ))}
        </AdminTable>
      )}
      <AdminPagination
        page={reviews.data.pagination.page}
        totalPages={reviews.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Reports
// ---------------------------------------------------------------------------

function ReportsTab() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState<ReportListParams>({ page: 1, page_size: 20 });
  const [error, setError] = useState<string | null>(null);
  const reports = useQuery({
    queryKey: moderationKeys.reports(params),
    queryFn: () => listModerationReports(params),
  });
  const mutation = useMutation({
    mutationFn: ({ id, action }: { id: string; action: "resolve" | "reject" }) =>
      resolveModerationReport(id, {
        action: action === "resolve" ? "hide" : undefined,
        action_reason: action === "resolve" ? "举报内容违规" : undefined,
        resolution_note: action === "resolve" ? "已确认违规，内容已隐藏" : "举报不成立，已驳回",
      }),
    onSuccess: async () => {
      setError(null);
      await queryClient.invalidateQueries({ queryKey: moderationKeys.reports(params) });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (reports.isPending) return <QueryLoading label="正在加载举报" />;
  if (reports.isError) return <QueryError message="举报列表加载失败" />;

  return (
    <div>
      <AdminToolbar>
        <Select
          value={params.status ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              status: (event.target.value || undefined) as ModerationReportStatus | undefined,
            }))
          }
        >
          <option value="">全部状态</option>
          <option value="open">待处理</option>
          <option value="reviewing">处理中</option>
          <option value="resolved">已解决</option>
          <option value="rejected">已驳回</option>
        </Select>
        <Select
          value={params.target_type ?? ""}
          onChange={(event) =>
            setParams((current) => ({
              ...current,
              page: 1,
              target_type: (event.target.value || undefined) as ModerationTargetType | undefined,
            }))
          }
        >
          <option value="">全部对象</option>
          <option value="topic">帖子</option>
          <option value="comment">评论</option>
          <option value="user">用户</option>
          <option value="file">文件</option>
        </Select>
      </AdminToolbar>
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      <AdminTable headers={["目标", "原因", "举报人", "优先级", "状态", "操作"]}>
        {reports.data.items.map((report: ReportItemV2) => (
          <tr key={report.id}>
            <td className="px-3 py-3">
              <div className="font-medium">
                {targetLabel(report.target_type)} · {report.target_id.slice(0, 8)}
              </div>
              {report.details ? (
                <div className="line-clamp-1 text-sm text-muted-foreground">{report.details}</div>
              ) : null}
            </td>
            <td className="px-3 py-3">{report.reason_code ?? report.reason}</td>
            <td className="px-3 py-3">@{report.reporter_username}</td>
            <td className="px-3 py-3">
              <span
                className={`rounded-sm px-1.5 py-0.5 text-xs ${
                  report.priority === "critical" || report.priority === "high"
                    ? "bg-destructive/10 text-destructive"
                    : "bg-muted text-muted-foreground"
                }`}
              >
                {report.priority}
              </span>
            </td>
            <td className="px-3 py-3">{report.status}</td>
            <td className="px-3 py-3">
              {report.status === "open" || report.status === "reviewing" ? (
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    disabled={mutation.isPending}
                    onClick={() => mutation.mutate({ id: report.id, action: "resolve" })}
                  >
                    接受并隐藏
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={mutation.isPending}
                    onClick={() => mutation.mutate({ id: report.id, action: "reject" })}
                  >
                    驳回
                  </Button>
                </div>
              ) : (
                <span className="text-xs text-muted-foreground">{report.resolution_note}</span>
              )}
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={reports.data.pagination.page}
        totalPages={reports.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}

function targetLabel(target: ModerationTargetType): string {
  return { topic: "帖子", comment: "评论", user: "用户", file: "文件" }[target];
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

function CasesTab() {
  const [params, setParams] = useState({ status: "open", page: 1, page_size: 20 });
  const cases = useQuery({
    queryKey: moderationKeys.cases(params),
    queryFn: () => listModerationCases(params),
  });
  if (cases.isPending) return <QueryLoading label="正在加载案件" />;
  if (cases.isError) return <QueryError message="案件加载失败" />;
  return (
    <div>
      <AdminToolbar>
        <Select
          value={params.status}
          onChange={(event) =>
            setParams((current) => ({ ...current, page: 1, status: event.target.value }))
          }
        >
          <option value="open">未结案件</option>
          <option value="reviewing">处理中</option>
          <option value="closed">已关闭</option>
        </Select>
      </AdminToolbar>
      {cases.data.items.length === 0 ? (
        <p className="border-y border-border py-12 text-center text-sm text-muted-foreground">
          暂无案件
        </p>
      ) : (
        <AdminTable headers={["目标", "来源", "优先级", "风险", "打开时间", "状态"]}>
          {cases.data.items.map((item) => (
            <tr key={item.id}>
              <td className="px-3 py-3">
                {item.target_type} · {item.target_id.slice(0, 8)}
              </td>
              <td className="px-3 py-3">{item.source}</td>
              <td className="px-3 py-3">{item.priority}</td>
              <td className="px-3 py-3">{item.risk_score}</td>
              <td className="px-3 py-3">{formatDateTime(item.opened_at)}</td>
              <td className="px-3 py-3">{item.status}</td>
            </tr>
          ))}
        </AdminTable>
      )}
      <AdminPagination
        page={cases.data.pagination.page}
        totalPages={cases.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sanctions
// ---------------------------------------------------------------------------

function SanctionsTab() {
  const [params, setParams] = useState({ page: 1, page_size: 20 });
  const sanctions = useQuery({
    queryKey: moderationKeys.sanctions(params),
    queryFn: () => listModerationSanctions(params),
  });
  if (sanctions.isPending) return <QueryLoading label="正在加载处罚记录" />;
  if (sanctions.isError) return <QueryError message="处罚记录加载失败" />;
  return (
    <div>
      {sanctions.data.items.length === 0 ? (
        <p className="border-y border-border py-12 text-center text-sm text-muted-foreground">
          暂无处罚记录
        </p>
      ) : (
        <AdminTable headers={["用户", "类型", "原因", "时间范围", "状态", "执行人"]}>
          {sanctions.data.items.map((sanction: SanctionItem) => (
            <tr key={sanction.id}>
              <td className="px-3 py-3">@{sanction.username}</td>
              <td className="px-3 py-3">{sanctionLabel(sanction.sanction_type)}</td>
              <td className="max-w-md px-3 py-3">{sanction.reason}</td>
              <td className="px-3 py-3">
                {formatDateTime(sanction.starts_at)}
                {sanction.ends_at ? ` → ${formatDateTime(sanction.ends_at)}` : "（永久）"}
              </td>
              <td className="px-3 py-3">{sanction.status}</td>
              <td className="px-3 py-3">
                {sanction.issuer_username ? `@${sanction.issuer_username}` : "-"}
              </td>
            </tr>
          ))}
        </AdminTable>
      )}
      <AdminPagination
        page={sanctions.data.pagination.page}
        totalPages={sanctions.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}

function sanctionLabel(type: SanctionType): string {
  return {
    warning: "警告",
    content_restriction: "内容限制",
    mute: "禁言",
    suspension: "停用",
    ban: "封禁",
  }[type];
}

// ---------------------------------------------------------------------------
// Rules (sensitive words + auto-moderation)
// ---------------------------------------------------------------------------

function RulesTab() {
  const queryClient = useQueryClient();
  const [params, setParams] = useState({ page: 1, page_size: 20 });
  const [error, setError] = useState<string | null>(null);
  const rules = useQuery({
    queryKey: moderationKeys.rules(params),
    queryFn: () => listModerationRules(params),
  });
  const toggle = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      updateModerationRule(id, { enabled }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: moderationKeys.rules(params) });
    },
    onError: (err) => setError(errorMessage(err)),
  });
  const [newRule, setNewRule] = useState<RuleRequest>({
    name: "",
    rule_type: "keyword",
    target_type: "topic",
    action: "flag",
    risk_score: 40,
    enabled: true,
    config: { keywords: [] },
  });

  const create = useMutation({
    mutationFn: () => createModerationRule(newRule),
    onSuccess: async () => {
      setError(null);
      setNewRule({ ...newRule, name: "", config: { keywords: [] } });
      await queryClient.invalidateQueries({ queryKey: moderationKeys.rules(params) });
    },
    onError: (err) => setError(errorMessage(err)),
  });

  if (rules.isPending) return <QueryLoading label="正在加载规则" />;
  if (rules.isError) return <QueryError message="规则加载失败" />;

  const keywordsText =
    Array.isArray(newRule.config.keywords) && newRule.config.keywords.length > 0
      ? String(newRule.config.keywords.join("，"))
      : "";

  return (
    <div>
      {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
      <div className="mb-5 rounded-md border border-border bg-white p-4">
        <h3 className="mb-3 text-sm font-semibold">添加敏感词规则</h3>
        <div className="grid gap-3 sm:grid-cols-2">
          <Input
            placeholder="规则名称（如：涉政敏感词）"
            value={newRule.name}
            onChange={(event) => setNewRule((r) => ({ ...r, name: event.target.value }))}
          />
          <Input
            placeholder="敏感词，用逗号分隔"
            value={keywordsText}
            onChange={(event) =>
              setNewRule((r) => ({
                ...r,
                config: {
                  ...r.config,
                  keywords: event.target.value
                    .split(/[,，]/)
                    .map((v) => v.trim())
                    .filter(Boolean),
                },
              }))
            }
          />
          <Select
            value={newRule.target_type}
            onChange={(event) => setNewRule((r) => ({ ...r, target_type: event.target.value }))}
          >
            <option value="all">全部内容</option>
            <option value="topic">帖子</option>
            <option value="comment">评论</option>
          </Select>
          <Select
            value={newRule.action}
            onChange={(event) =>
              setNewRule((r) => ({ ...r, action: event.target.value as RuleRequest["action"] }))
            }
          >
            <option value="flag">标记（进待审）</option>
            <option value="hide">隐藏</option>
            <option value="reject">拒绝发布</option>
          </Select>
        </div>
        <Button
          type="button"
          size="sm"
          className="mt-3"
          disabled={create.isPending || !newRule.name.trim() || keywordsText.length === 0}
          onClick={() => create.mutate()}
        >
          {create.isPending ? "创建中…" : "创建规则"}
        </Button>
      </div>

      <AdminTable headers={["规则", "类型", "目标", "动作", "风险", "命中", "状态", "操作"]}>
        {rules.data.items.map((rule: RuleItem) => (
          <tr key={rule.id}>
            <td className="px-3 py-3">
              <div className="font-medium">{rule.name}</div>
              {rule.rule_type === "keyword" ? (
                <div className="text-xs text-muted-foreground">
                  {String(
                    Array.isArray(rule.config.keywords) ? rule.config.keywords.join("，") : "",
                  ).slice(0, 60)}
                </div>
              ) : null}
            </td>
            <td className="px-3 py-3">{rule.rule_type}</td>
            <td className="px-3 py-3">{rule.target_type}</td>
            <td className="px-3 py-3">{rule.action}</td>
            <td className="px-3 py-3">{rule.risk_score}</td>
            <td className="px-3 py-3">{rule.hit_count}</td>
            <td className="px-3 py-3">{rule.enabled ? "启用" : "停用"}</td>
            <td className="px-3 py-3">
              <Button
                size="sm"
                variant="outline"
                disabled={toggle.isPending}
                onClick={() => toggle.mutate({ id: rule.id, enabled: !rule.enabled })}
              >
                {rule.enabled ? "停用" : "启用"}
              </Button>
            </td>
          </tr>
        ))}
      </AdminTable>
      <AdminPagination
        page={rules.data.pagination.page}
        totalPages={rules.data.pagination.total_pages}
        onPageChange={(page) => setParams((current) => ({ ...current, page }))}
      />
    </div>
  );
}
