import { SteamAuthComplete } from "@/components/auth/steam-auth-complete";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("Steam 认证");

export default function SteamAuthCompletePage() {
  return <SteamAuthComplete />;
}
