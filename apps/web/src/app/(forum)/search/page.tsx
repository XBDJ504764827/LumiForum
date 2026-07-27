import type { Metadata } from "next";
import { Suspense } from "react";

import { SearchView } from "@/components/forum/search-view";
import { QueryLoading } from "@/components/forum/query-state";

export const metadata: Metadata = {
  title: "搜索 | LumiForum",
};

export default function SearchPage() {
  return (
    <Suspense fallback={<QueryLoading label="正在加载搜索" />}>
      <SearchView />
    </Suspense>
  );
}
