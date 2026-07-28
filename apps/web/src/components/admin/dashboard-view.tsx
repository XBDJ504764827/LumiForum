"use client";

import { useQuery } from "@tanstack/react-query";
import {
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
import { adminKeys, getAdminDashboard } from "@/lib/api/admin";

export function AdminDashboardView() {
  const query = useQuery({
    queryKey: adminKeys.dashboard,
    queryFn: getAdminDashboard,
  });

  if (query.isPending) return <QueryLoading label="正在加载仪表盘" />;
  if (query.isError || !query.data) return <QueryError message="无法加载仪表盘" />;

  const data = query.data;
  const cards = [
    { label: "用户总数", value: data.users_total },
    { label: "帖子总数", value: data.topics_total },
    { label: "评论总数", value: data.comments_total },
    { label: "文件总数", value: data.uploads_total },
    { label: "今日注册", value: data.users_today },
    { label: "今日发帖", value: data.topics_today },
    { label: "7 日活跃", value: data.active_users_7d },
    { label: "待处理举报", value: data.reports_open },
  ];

  return (
    <div>
      <AdminPageHeader title="仪表盘" description="社区关键指标与近期趋势。" />
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {cards.map((card) => (
          <div key={card.label} className="border border-border bg-white px-4 py-4">
            <p className="text-sm text-muted-foreground">{card.label}</p>
            <p className="mt-2 text-2xl font-semibold tabular-nums">{card.value}</p>
          </div>
        ))}
      </div>

      <div className="mt-6 grid gap-6 xl:grid-cols-2">
        <ChartCard title="近 7 日注册">
          <ResponsiveContainer width="100%" height={240}>
            <LineChart data={data.registrations_7d}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 12 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Line type="monotone" dataKey="count" stroke="#2563eb" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
        <ChartCard title="近 7 日发帖">
          <ResponsiveContainer width="100%" height={240}>
            <LineChart data={data.topics_7d}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 12 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Line type="monotone" dataKey="count" stroke="#059669" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
      </div>

      <section className="mt-6 border border-border bg-white">
        <div className="border-b border-border px-4 py-3 text-sm font-medium">热门帖子</div>
        <ul className="divide-y divide-border">
          {data.hot_topics.map((topic) => (
            <li
              key={topic.id}
              className="flex items-center justify-between gap-4 px-4 py-3 text-sm"
            >
              <div className="min-w-0">
                <p className="truncate font-medium">{topic.title}</p>
                <p className="text-muted-foreground">/{topic.slug}</p>
              </div>
              <div className="shrink-0 text-right text-muted-foreground">
                <p>浏览 {topic.view_count}</p>
                <p>
                  回复 {topic.reply_count} · 点赞 {topic.like_count}
                </p>
              </div>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="border border-border bg-white p-4">
      <h2 className="mb-4 text-sm font-medium">{title}</h2>
      {children}
    </section>
  );
}
