import type { Metadata } from "next";

import { RequireAuth } from "@/components/auth/route-guards";
import { FavoritesView } from "@/components/forum/favorites-view";

export const metadata: Metadata = {
  title: "我的收藏 | LumiForum",
};

export default function FavoritesPage() {
  return (
    <RequireAuth>
      <FavoritesView />
    </RequireAuth>
  );
}
