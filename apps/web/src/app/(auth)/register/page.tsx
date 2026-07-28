import { RegisterForm } from "@/components/auth/register-form";
import { privatePageMetadata } from "@/lib/seo/metadata";

export const metadata = privatePageMetadata("注册", "创建 LumiForum 账户");

export default function RegisterPage() {
  return <RegisterForm />;
}
