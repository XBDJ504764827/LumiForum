"use client";

import { Button } from "@lumiforum/ui";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { AdminPageHeader } from "@/components/admin/admin-table";
import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { adminKeys, getAdminAnalytics } from "@/lib/api/admin";

const windows = [7, 30, 90];

export function AdminAnalyticsView() {
  const [days, setDays] = useState(30);
  const analytics = useQuery({
    queryKey: adminKeys.analytics(days),
    queryFn: () => getAdminAnalytics(days),
  });

  if (analytics.isPending) return <QueryLoading label="正在加载数据" />;
  if (analytics.isError || !analytics.data) return <QueryError message="数据分析加载失败" />;

  const data = analytics.data;
  const activity = data.topics.map((item, index) => ({
    date: item.date,
    topics: item.count,
    comments: data.comments[index]?.count ?? 0,
    polls: data.polls[index]?.count ?? 0,
  }));

  return (
    <div>
      <AdminPageHeader
        title="数据分析"
        description="社区增长与内容趋势。"
        actions={
          <div className="inline-flex rounded-md border border-border p-0.5">
            {windows.map((window) => (
              <Button
                key={window}
                type="button"
                size="sm"
                variant={days === window ? "default" : "ghost"}
                onClick={() => setDays(window)}
              >
                {window} 天
              </Button>
            ))}
          </div>
        }
      />

      <div className="grid gap-6 xl:grid-cols-2">
        <ChartCard title="每日注册">
          <ResponsiveContainer width="100%" height={240}>
            <BarChart data={data.registrations}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 11 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Bar dataKey="count" fill="#2563eb" radius={[3, 3, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </ChartCard>

        <ChartCard title="用户增长（累计）">
          <ResponsiveContainer width="100%" height={240}>
            <AreaChart data={data.cumulative_users}>
              <defs>
                <linearGradient id="cumUsers" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#16a34a" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="#16a34a" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 11 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Area type="monotone" dataKey="count" stroke="#16a34a" fill="url(#cumUsers)" />
            </AreaChart>
          </ResponsiveContainer>
        </ChartCard>

        <ChartCard title="内容产出（帖子 / 评论 / 投票）">
          <ResponsiveContainer width="100%" height={240}>
            <LineChart data={activity}>
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis dataKey="date" tick={{ fontSize: 11 }} />
              <YAxis allowDecimals={false} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Legend />
              <Line type="monotone" dataKey="topics" stroke="#2563eb" strokeWidth={2} dot={false} />
              <Line
                type="monotone"
                dataKey="comments"
                stroke="#dc2626"
                strokeWidth={2}
                dot={false}
              />
              <Line type="monotone" dataKey="polls" stroke="#d97706" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>

        <ChartCard title="热门分类（帖子数）">
          <ResponsiveContainer width="100%" height={240}>
            <BarChart data={data.hot_categories} layout="vertical">
              <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
              <XAxis type="number" allowDecimals={false} tick={{ fontSize: 12 }} />
              <YAxis type="category" dataKey="name" width={90} tick={{ fontSize: 12 }} />
              <Tooltip />
              <Bar dataKey="topic_count" name="帖子" fill="#7c3aed" radius={[0, 3, 3, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </ChartCard>

        <ChartCard title="热门帖子" full>
          <ul className="divide-y divide-border text-sm">
            {data.hot_topics.map((topic, index) => (
              <li key={topic.id} className="flex items-center justify-between gap-3 py-2">
                <span className="flex min-w-0 items-center gap-2">
                  <span className="w-5 shrink-0 text-center text-xs tabular-nums text-muted-foreground">
                    {index + 1}
                  </span>
                  <a
                    href={`/topics/${topic.slug}`}
                    target="_blank"
                    rel="noreferrer"
                    className="truncate hover:text-primary"
                  >
                    {topic.title}
                  </a>
                </span>
                <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                  {topic.view_count} 浏览 · {topic.like_count} 赞 · {topic.reply_count} 回复
                </span>
              </li>
            ))}
          </ul>
        </ChartCard>
      </div>
    </div>
  );
}

function ChartCard({
  title,
  children,
  full = false,
}: {
  title: string;
  children: React.ReactNode;
  full?: boolean;
}) {
  return (
    <div className={`border border-border bg-white p-4 ${full ? "xl:col-span-2" : ""}`}>
      <h3 className="mb-3 text-sm font-medium">{title}</h3>
      {children}
    </div>
  );
}
