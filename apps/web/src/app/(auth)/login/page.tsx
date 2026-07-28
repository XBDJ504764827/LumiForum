import { LoginForm } from "@/components/auth/login-form";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("登录", "登录 LumiForum 账户");

export default function LoginPage() {
  return <LoginForm />;
}
