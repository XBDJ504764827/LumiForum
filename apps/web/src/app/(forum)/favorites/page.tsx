import { RequireAuth } from "@/components/auth/route-guards";
import { FavoritesView } from "@/components/forum/favorites-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("我的收藏");

export default function FavoritesPage() {
  return (
    <RequireAuth>
      <FavoritesView />
    </RequireAuth>
  );
}
