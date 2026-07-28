import type { ReactNode } from "react";

import { Providers } from "@/components/providers";
import { rootMetadata } from "@/lib/seo/metadata";

import "./globals.css";

export const metadata = rootMetadata();

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="zh-CN" suppressHydrationWarning>
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
