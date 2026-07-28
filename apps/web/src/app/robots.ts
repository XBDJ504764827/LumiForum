import type { MetadataRoute } from "next";

import { absoluteUrl } from "@/lib/seo/site";

export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", "/categories", "/topics", "/search", "/rss.xml"],
        disallow: [
          "/admin",
          "/admin/",
          "/profile",
          "/login",
          "/register",
          "/favorites",
          "/notifications",
          "/topics/new",
          "/*/edit",
        ],
      },
    ],
    sitemap: absoluteUrl("/sitemap.xml"),
    host: absoluteUrl("/").replace(/^https?:\/\//, ""),
  };
}
