"use client";

import type { AdminDashboardRange } from "@lumiforum/types";
import { Button } from "@lumiforum/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { AdminPageHeader } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { adminKeys, getAdminDashboardRange } from "@/lib/api/admin";

const ranges: Array<{ value: AdminDashboardRange; label: string }> = [
  { value: "today", label: "今日" },
  { value: "7d", label: "7 天" },
  { value: "30d", label: "30 天" },
];

export function AdminDashboardView() {
  const [range, setRange] = useState<AdminDashboardRange>("7d");
  const query = useQuery({
    queryKey: adminKeys.dashboardRange(range),
    queryFn: () => getAdminDashboardRange(range),
  });

  if (query.isPending) return <QueryLoading label="正在加载仪表盘" />;
  if (query.isError || !query.data) return <QueryError message="无法加载仪表盘" />;

  const data = query.data;
  const users = [
    { label: "用户总数", value: data.users_total },
    { label: "今日新增", value: data.users_today },
    { label: "今日活跃", value: data.active_users_today },
    { label: "在线用户", value: data.online_users },
  ];
  const content = [
    { label: "帖子", value: data.topics_total, today: data.topics_today },
    { label: "评论", value: data.comments_total, today: data.comments_today },
    { label: "投票", value: data.polls_total },
    { label: "文件", value: data.uploads_total },
    { label: "存储用量", value: formatBytes(data.storage_bytes) },
    { label: "待处理举报", value: data.reports_open },
  ];
  const system = [
    { label: "API 请求", value: data.api_requests_total },
    { label: "WebSocket 连接", value: data.ws_connections },
    { label: "7 日活跃", value: data.active_users_7d },
    { label: "举报总数", value: data.reports_total },
  ];

  return (
    <div>
      <AdminPageHeader
        title="仪表盘"
        description="社区关键指标与趋势。"
        actions={
          <div className="inline-flex rounded-md border border-border p-0.5">
            {ranges.map((item) => (
              <Button
                key={item.value}
                type="button"
                size="sm"
                variant={range === item.value ? "default" : "ghost"}
                onClick={() => setRange(item.value)}
              >
                {item.label}
              </Button>
            ))}
          </div>
        }
      />

      <MetricGroup title="用户数据">
        {users.map((card) => (
          <MetricCard key={card.label} label={card.label} value={card.value} />
        ))}
      </MetricGroup>
      <MetricGroup title="内容数据">
        {content.map((card) => (
          <MetricCard key={card.label} label={card.label} value={card.value} />
        ))}
      </MetricGroup>
      <MetricGroup title="系统数据">
        {system.map((card) => (
          <MetricCard key={card.label} label={card.label} value={card.value} />
        ))}
      </MetricGroup>

      <div className="mt-6 grid gap-6 xl:grid-cols-2">
        <ChartCard title={`近 ${trendDays(data.range)} 日注册`}>
          <ResponsiveContainer width="100%" height={220}>
            <LineChart data={data.registrations}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 12 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Line type="monotone" dataKey="count" stroke="#2563eb" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
        <ChartCard title={`近 ${trendDays(data.range)} 日发帖`}>
          <ResponsiveContainer width="100%" height={220}>
            <LineChart data={data.topics}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 12 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Line type="monotone" dataKey="count" stroke="#16a34a" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
        <ChartCard title="热门分类">
          <ResponsiveContainer width="100%" height={220}>
            <BarChart data={data.hot_categories}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="name" tick={{ fontSize: 11 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Bar dataKey="topic_count" name="帖子" fill="#2563eb" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </ChartCard>
        <ChartCard title="热门帖子">
          <ul className="divide-y divide-border">
            {data.hot_topics.slice(0, 6).map((topic) => (
              <li key={topic.id} className="flex items-center justify-between gap-3 py-2 text-sm">
                <a
                  href={`/topics/${topic.slug}`}
                  target="_blank"
                  rel="noreferrer"
                  className="truncate hover:text-primary"
                >
                  {topic.title}
                </a>
                <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                  {topic.view_count} 浏览
                </span>
              </li>
            ))}
          </ul>
        </ChartCard>
      </div>
    </div>
  );
}

function MetricGroup({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mt-6">
      <h2 className="mb-2 text-sm font-medium text-muted-foreground">{title}</h2>
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{children}</div>
    </div>
  );
}

function MetricCard({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="border border-border bg-white px-4 py-4">
      <p className="text-sm text-muted-foreground">{label}</p>
      <p className="mt-2 text-2xl font-semibold tabular-nums">{value}</p>
    </div>
  );
}

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border border-border bg-white p-4">
      <h3 className="mb-3 text-sm font-medium">{title}</h3>
      {children}
    </div>
  );
}

function trendDays(range: AdminDashboardRange): number {
  if (range === "today") return 1;
  if (range === "30d") return 30;
  return 7;
}

function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}
