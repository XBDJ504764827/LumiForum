import type { Metadata } from "next";
import { Suspense } from "react";

import { QueryLoading } from "@/components/forum/query-state";
import { SearchView } from "@/components/forum/search-view";
import { searchMetadata } from "@/lib/seo/metadata";

type Props = {
  searchParams: Promise<{ q?: string }>;
};

export async function generateMetadata({ searchParams }: Props): Promise<Metadata> {
  const query = await searchParams;
  return searchMetadata(query.q);
}

export default function SearchPage() {
  return (
    <Suspense fallback={<QueryLoading label="正在加载搜索" />}>
      <SearchView />
    </Suspense>
  );
}
