import type { Metadata } from "next";

import { CategoryDirectory } from "@/components/forum/category-directory";

export const metadata: Metadata = {
  title: "板块 | LumiForum",
};

export default function CategoriesPage() {
  return <CategoryDirectory />;
}
