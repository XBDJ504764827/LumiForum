import type { Metadata } from "next";

import { ProfileView } from "@/components/profile/profile-view";
import { RequireAuth } from "@/components/auth/route-guards";

export const metadata: Metadata = {
  title: "个人中心 | LumiForum",
};

export default function ProfilePage() {
  return (
    <RequireAuth>
      <ProfileView />
    </RequireAuth>
  );
}
