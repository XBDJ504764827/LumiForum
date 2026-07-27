"use client";

import type { Route } from "next";
import type {
  Category,
  CommentSearchHit,
  SearchHit,
  SearchSort,
  SearchType,
  TopicSearchHit,
  UserSearchHit,
} from "@lumiforum/types";
import { Avatar, AvatarFallback, AvatarImage, Badge, Button, Input } from "@lumiforum/ui";
import { useQuery } from "@tanstack/react-query";
import {
  Clock3,
  Flame,
  MessageSquare,
  Search as SearchIcon,
  TrendingUp,
  UserRound,
  X,
} from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useMemo, useState, type FormEvent } from "react";

import { QueryError, QueryLoading } from "@/components/forum/query-state";
import { listCategories } from "@/lib/api/forum";
import {
  clearRecentSearches,
  hotKeywords,
  loadRecentSearches,
  saveRecentSearch,
  search,
  searchKeys,
  searchSuggestions,
} from "@/lib/api/search";

const PAGE_SIZE = 20;

export function SearchView() {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const qParam = searchParams.get("q")?.trim() ?? "";
  const typeParam = (searchParams.get("type") as SearchType | null) ?? "topic";
  const sortParam = (searchParams.get("sort") as SearchSort | null) ?? "relevance";
  const pageParam = Math.max(1, Number(searchParams.get("page") ?? "1") || 1);
  const categoryParam = searchParams.get("category_id") ?? "";

  const [draft, setDraft] = useState(qParam);
  const [draftSource, setDraftSource] = useState(qParam);
  const [recent, setRecent] = useState<string[]>(() => loadRecentSearches());
  if (draftSource !== qParam) {
    setDraftSource(qParam);
    setDraft(qParam);
  }

  const categories = useQuery({
    queryKey: ["forum", "categories"],
    queryFn: listCategories,
    staleTime: 60_000,
  });

  const params = useMemo(
    () => ({
      q: qParam,
      type: typeParam,
      sort: sortParam,
      page: pageParam,
      page_size: PAGE_SIZE,
      category_id: categoryParam || undefined,
    }),
    [qParam, typeParam, sortParam, pageParam, categoryParam],
  );

  const results = useQuery({
    queryKey: searchKeys.results(params),
    queryFn: () => search(params),
    enabled: qParam.length > 0,
  });

  const suggestions = useQuery({
    queryKey: searchKeys.suggestions(draft.trim()),
    queryFn: () => searchSuggestions(draft.trim()),
    enabled: draft.trim().length >= 1 && draft.trim() !== qParam,
    staleTime: 10_000,
  });

  const hot = useQuery({
    queryKey: searchKeys.hot,
    queryFn: hotKeywords,
    staleTime: 30_000,
  });

  const pushSearch = (next: {
    q?: string;
    type?: SearchType;
    sort?: SearchSort;
    page?: number;
    category_id?: string;
  }) => {
    const query = new URLSearchParams();
    const q = (next.q ?? qParam).trim();
    if (q) query.set("q", q);
    query.set("type", next.type ?? typeParam);
    query.set("sort", next.sort ?? sortParam);
    const page = next.page ?? 1;
    if (page > 1) query.set("page", String(page));
    const categoryId = next.category_id ?? categoryParam;
    if (categoryId) query.set("category_id", categoryId);
    const href = query.size ? `${pathname}?${query}` : pathname;
    router.push(href as Route);
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const value = draft.trim();
    if (!value) return;
    setRecent(saveRecentSearch(value));
    pushSearch({ q: value, page: 1 });
  };

  return (
    <main className="mx-auto max-w-5xl px-5 py-9 sm:px-8">
      <div className="mb-8 border-b border-border pb-6">
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Link href="/" className="hover:text-foreground">
            首页
          </Link>
          <span>/</span>
          <span>搜索</span>
        </div>
        <h1 className="mt-3 flex items-center gap-2 text-3xl font-semibold">
          <SearchIcon className="size-7" aria-hidden="true" />
          搜索
        </h1>
        <form className="mt-5 flex flex-col gap-3 sm:flex-row" onSubmit={submit}>
          <Input
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="搜索帖子、评论或用户"
            className="h-11"
            aria-label="搜索关键词"
          />
          <Button type="submit" className="h-11 gap-2 sm:w-28">
            <SearchIcon className="size-4" aria-hidden="true" />
            搜索
          </Button>
        </form>
      </div>

      <div className="grid gap-8 lg:grid-cols-[220px_minmax(0,1fr)]">
        <aside className="space-y-6">
          <FilterGroup title="类型">
            {(
              [
                ["topic", "帖子"],
                ["comment", "评论"],
                ["user", "用户"],
              ] as const
            ).map(([value, label]) => (
              <FilterButton
                key={value}
                active={typeParam === value}
                onClick={() => pushSearch({ type: value, page: 1 })}
              >
                {label}
              </FilterButton>
            ))}
          </FilterGroup>

          <FilterGroup title="排序">
            {(
              [
                ["relevance", "相关度"],
                ["latest", "最新"],
                ["hot", "热门"],
              ] as const
            ).map(([value, label]) => (
              <FilterButton
                key={value}
                active={sortParam === value}
                onClick={() => pushSearch({ sort: value, page: 1 })}
              >
                {label}
              </FilterButton>
            ))}
          </FilterGroup>

          {typeParam === "topic" ? (
            <FilterGroup title="分类">
              <FilterButton
                active={!categoryParam}
                onClick={() => pushSearch({ category_id: "", page: 1 })}
              >
                全部
              </FilterButton>
              {(categories.data ?? []).map((category: Category) => (
                <FilterButton
                  key={category.id}
                  active={categoryParam === category.id}
                  onClick={() => pushSearch({ category_id: category.id, page: 1 })}
                >
                  {category.name}
                </FilterButton>
              ))}
            </FilterGroup>
          ) : null}
        </aside>

        <section>
          {!qParam ? (
            <EmptySearchState
              recent={recent}
              hot={hot.data?.keywords.map((item) => item.keyword) ?? []}
              onPick={(value) => {
                setDraft(value);
                setRecent(saveRecentSearch(value));
                pushSearch({ q: value, page: 1 });
              }}
              onClearRecent={() => {
                clearRecentSearches();
                setRecent([]);
              }}
            />
          ) : results.isPending ? (
            <QueryLoading label="正在搜索" />
          ) : results.isError ? (
            <QueryError message="搜索失败，请稍后重试" />
          ) : results.data.items.length === 0 ? (
            <div className="border-y border-border py-14 text-center">
              <p className="text-sm text-muted-foreground">没有找到与「{qParam}」相关的结果</p>
              {suggestions.data?.suggestions.length ? (
                <div className="mt-4 flex flex-wrap justify-center gap-2">
                  {suggestions.data.suggestions.map((item) => (
                    <button
                      key={item}
                      type="button"
                      className="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                      onClick={() => {
                        setDraft(item);
                        setRecent(saveRecentSearch(item));
                        pushSearch({ q: item, page: 1 });
                      }}
                    >
                      {item}
                    </button>
                  ))}
                </div>
              ) : null}
            </div>
          ) : (
            <>
              <p className="mb-4 text-sm text-muted-foreground">
                「{results.data.query}」共 {results.data.pagination.total} 条结果
              </p>
              <ul className="divide-y divide-border border-y border-border">
                {results.data.items.map((item) => (
                  <li key={`${item.kind}-${item.id}`} className="py-5">
                    <SearchResultItem hit={item} />
                  </li>
                ))}
              </ul>
              <Pagination
                page={results.data.pagination.page}
                totalPages={results.data.pagination.total_pages}
                onChange={(page) => pushSearch({ page })}
              />
            </>
          )}

          {qParam && suggestions.data?.suggestions.length && draft.trim() !== qParam ? (
            <div className="mt-6 rounded-md border border-border p-4">
              <p className="mb-2 text-sm font-medium">搜索建议</p>
              <div className="flex flex-wrap gap-2">
                {suggestions.data.suggestions.map((item) => (
                  <button
                    key={item}
                    type="button"
                    className="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                    onClick={() => {
                      setDraft(item);
                      setRecent(saveRecentSearch(item));
                      pushSearch({ q: item, page: 1 });
                    }}
                  >
                    {item}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        </section>
      </div>
    </main>
  );
}

function SearchResultItem({ hit }: { hit: SearchHit }) {
  if (hit.kind === "topic") return <TopicHit item={hit} />;
  if (hit.kind === "comment") return <CommentHit item={hit} />;
  return <UserHit item={hit} />;
}

function TopicHit({ item }: { item: TopicSearchHit & { kind: "topic" } }) {
  return (
    <article>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <Badge className="bg-muted text-foreground">帖子</Badge>
        <Link
          href={`/categories/${item.category.slug}`}
          className="text-xs font-medium text-primary hover:underline"
        >
          {item.category.name}
        </Link>
      </div>
      <h2 className="text-lg font-semibold">
        <Link href={`/topics/${item.slug}`} className="hover:text-primary">
          {item.title}
        </Link>
      </h2>
      <p className="mt-1 line-clamp-2 text-sm text-muted-foreground">
        {item.highlight || item.summary || "暂无摘要"}
      </p>
      <div className="mt-3 flex flex-wrap gap-3 text-xs text-muted-foreground">
        <span>{item.author.nickname || item.author.username}</span>
        <span>{formatDate(item.created_at)}</span>
        <span className="inline-flex items-center gap-1">
          <TrendingUp className="size-3.5" />
          {item.stats.views}
        </span>
        <span className="inline-flex items-center gap-1">
          <MessageSquare className="size-3.5" />
          {item.stats.replies}
        </span>
      </div>
    </article>
  );
}

function CommentHit({ item }: { item: CommentSearchHit & { kind: "comment" } }) {
  return (
    <article>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <Badge className="bg-muted text-foreground">评论</Badge>
        <Link href={`/topics/${item.topic_slug}`} className="text-xs text-primary hover:underline">
          {item.topic_title}
        </Link>
      </div>
      <p className="text-sm leading-6 text-foreground">{item.highlight || item.content_preview}</p>
      <div className="mt-3 flex flex-wrap gap-3 text-xs text-muted-foreground">
        <span>{item.author.nickname || item.author.username}</span>
        <span>{formatDate(item.created_at)}</span>
        <Link
          href={`/topics/${item.topic_slug}#comment-${item.id}`}
          className="text-primary hover:underline"
        >
          查看评论
        </Link>
      </div>
    </article>
  );
}

function UserHit({ item }: { item: UserSearchHit & { kind: "user" } }) {
  return (
    <article className="flex items-center justify-between gap-4">
      <div className="flex min-w-0 items-center gap-3">
        <Avatar className="size-10 border border-border">
          {item.avatar ? <AvatarImage src={item.avatar} alt="" /> : null}
          <AvatarFallback>
            {(item.nickname || item.username).slice(0, 2).toUpperCase()}
          </AvatarFallback>
        </Avatar>
        <div className="min-w-0">
          <p className="font-medium">{item.nickname || item.username}</p>
          <p className="text-xs text-muted-foreground">
            @{item.username} · {item.followers_count} 粉丝
          </p>
        </div>
      </div>
      <Link
        href={`/users/${item.id}/followers`}
        className="inline-flex h-9 items-center gap-2 rounded-md border border-border px-3 text-sm hover:bg-muted"
      >
        <UserRound className="size-4" />
        查看
      </Link>
    </article>
  );
}

function EmptySearchState({
  recent,
  hot,
  onPick,
  onClearRecent,
}: {
  recent: string[];
  hot: string[];
  onPick: (value: string) => void;
  onClearRecent: () => void;
}) {
  return (
    <div className="space-y-8">
      <div className="border-y border-border py-10 text-center text-sm text-muted-foreground">
        输入关键词开始搜索帖子、评论或用户
      </div>
      {recent.length > 0 ? (
        <section>
          <div className="mb-3 flex items-center justify-between">
            <h2 className="flex items-center gap-2 text-sm font-medium">
              <Clock3 className="size-4" />
              最近搜索
            </h2>
            <button
              type="button"
              className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
              onClick={onClearRecent}
            >
              <X className="size-3.5" />
              清空
            </button>
          </div>
          <div className="flex flex-wrap gap-2">
            {recent.map((item) => (
              <button
                key={item}
                type="button"
                className="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                onClick={() => onPick(item)}
              >
                {item}
              </button>
            ))}
          </div>
        </section>
      ) : null}
      {hot.length > 0 ? (
        <section>
          <h2 className="mb-3 flex items-center gap-2 text-sm font-medium">
            <Flame className="size-4" />
            热门搜索
          </h2>
          <div className="flex flex-wrap gap-2">
            {hot.map((item) => (
              <button
                key={item}
                type="button"
                className="rounded-md border border-border px-3 py-1.5 text-sm hover:bg-muted"
                onClick={() => onPick(item)}
              >
                {item}
              </button>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}

function FilterGroup({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <p className="mb-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
        {title}
      </p>
      <div className="flex flex-wrap gap-2 lg:flex-col">{children}</div>
    </div>
  );
}

function FilterButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-md px-3 py-1.5 text-left text-sm ${
        active ? "bg-primary text-primary-foreground" : "border border-border hover:bg-muted"
      }`}
    >
      {children}
    </button>
  );
}

function Pagination({
  page,
  totalPages,
  onChange,
}: {
  page: number;
  totalPages: number;
  onChange: (page: number) => void;
}) {
  if (totalPages <= 1) return null;
  return (
    <div className="mt-6 flex items-center justify-center gap-2">
      <Button variant="outline" size="sm" disabled={page <= 1} onClick={() => onChange(page - 1)}>
        上一页
      </Button>
      <span className="text-sm text-muted-foreground">
        {page} / {totalPages}
      </span>
      <Button
        variant="outline"
        size="sm"
        disabled={page >= totalPages}
        onClick={() => onChange(page + 1)}
      >
        下一页
      </Button>
    </div>
  );
}

function formatDate(value: string): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}
