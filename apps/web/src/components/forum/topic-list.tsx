import type { TopicSummary } from "@lumiforum/types";
import { Badge } from "@lumiforum/ui";
import { BarChart3, Eye, Heart, MessageSquare, Pin, Sparkles } from "lucide-react";
import Link from "next/link";

export function TopicList({ topics }: { topics: TopicSummary[] }) {
  if (topics.length === 0) {
    return (
      <div className="border-y border-border py-14 text-center text-sm text-muted-foreground">
        暂无帖子
      </div>
    );
  }

  return (
    <div className="divide-y divide-border border-y border-border">
      {topics.map((topic) => (
        <article key={topic.id} className="py-5 first:pt-4">
          <div className="flex min-w-0 items-start justify-between gap-5">
            <div className="min-w-0 flex-1">
              <div className="mb-2 flex flex-wrap items-center gap-2">
                {topic.is_pinned ? (
                  <Badge className="gap-1 bg-foreground text-white">
                    <Pin className="size-3" aria-hidden="true" />
                    置顶
                  </Badge>
                ) : null}
                {topic.is_featured ? (
                  <Badge className="gap-1 bg-accent/15 text-foreground">
                    <Sparkles className="size-3" aria-hidden="true" />
                    精华
                  </Badge>
                ) : null}
                {topic.has_poll ? (
                  <Badge className="gap-1 bg-primary/10 text-primary">
                    <BarChart3 className="size-3" aria-hidden="true" />
                    投票
                  </Badge>
                ) : null}
                <Link
                  href={`/categories/${topic.category.slug}`}
                  className="text-xs font-medium text-primary hover:underline"
                >
                  {topic.category.name}
                </Link>
              </div>
              <h3 className="text-lg font-semibold leading-7">
                <Link href={`/topics/${topic.slug}`} className="hover:text-primary">
                  {topic.title}
                </Link>
              </h3>
              {topic.summary ? (
                <p className="mt-1 line-clamp-2 text-sm leading-6 text-muted-foreground">
                  {topic.summary}
                </p>
              ) : null}
              <div className="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                <span>
                  {topic.author_anonymous
                    ? "匿名玩家"
                    : topic.author.nickname || topic.author.username}
                </span>
                {topic.author_anonymous ? <span>· 匿名发帖</span> : null}
                <span>{formatDate(topic.created_at)}</span>
              </div>
            </div>
            <div className="hidden shrink-0 grid-cols-3 gap-4 pt-2 text-xs text-muted-foreground sm:grid">
              <Stat icon={Eye} value={topic.stats.views} label="浏览" />
              <Stat icon={MessageSquare} value={topic.stats.replies} label="回复" />
              <Stat icon={Heart} value={topic.stats.likes} label="喜欢" />
            </div>
          </div>
        </article>
      ))}
    </div>
  );
}

function Stat({ icon: Icon, value, label }: { icon: typeof Eye; value: number; label: string }) {
  return (
    <span className="flex min-w-10 flex-col items-center gap-1" title={label}>
      <Icon className="size-4" aria-hidden="true" />
      {compactNumber(value)}
    </span>
  );
}

function compactNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", { notation: "compact" }).format(value);
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", { dateStyle: "medium" }).format(new Date(value));
}
