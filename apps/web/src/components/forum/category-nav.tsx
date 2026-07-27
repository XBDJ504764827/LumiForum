import type { Category } from "@lumiforum/types";
import { Layers3 } from "lucide-react";
import Link from "next/link";

export function CategoryNav({ categories }: { categories: Category[] }) {
  return (
    <nav aria-label="论坛板块">
      <div className="mb-4 flex items-center gap-2 text-sm font-semibold">
        <Layers3 className="size-4" aria-hidden="true" />
        板块
      </div>
      <div className="divide-y divide-border border-y border-border">
        {categories.map((category) => (
          <Link
            key={category.id}
            href={`/categories/${category.slug}`}
            className="flex items-center justify-between gap-3 py-3 text-sm hover:text-primary"
          >
            <span className="min-w-0 truncate font-medium">{category.name}</span>
            <span className="shrink-0 text-xs text-muted-foreground">{category.topic_count}</span>
          </Link>
        ))}
        {categories.length === 0 ? (
          <p className="py-5 text-sm text-muted-foreground">暂无可见板块</p>
        ) : null}
      </div>
      <Link href="/categories" className="mt-4 inline-block text-sm text-primary hover:underline">
        查看全部板块
      </Link>
    </nav>
  );
}
