import { RequireAuth } from "@/components/auth/route-guards";
import { ProfileView } from "@/components/profile/profile-view";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("个人中心");

export default function ProfilePage() {
  return (
    <RequireAuth>
      <ProfileView />
    </RequireAuth>
  );
}
