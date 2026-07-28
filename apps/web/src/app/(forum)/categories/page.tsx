import { CategoryDirectory } from "@/components/forum/category-directory";
import { categoriesIndexMetadata } from "@/lib/seo/metadata";

export const metadata = categoriesIndexMetadata();
export const revalidate = 120;

export default function CategoriesPage() {
  return <CategoryDirectory />;
}
