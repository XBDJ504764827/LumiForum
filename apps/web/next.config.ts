import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  allowedDevOrigins: ["192.168.0.138", "localhost"],
  transpilePackages: ["@lumiforum/ui", "@lumiforum/shared", "@lumiforum/types"],
  typedRoutes: true,
  poweredByHeader: false,
  compress: true,
  images: {
    remotePatterns: [
      { protocol: "http", hostname: "**" },
      { protocol: "https", hostname: "**" },
    ],
  },
  async headers() {
    // The browser talks to a separate API origin (NEXT_PUBLIC_API_URL) plus a
    // WebSocket on the same host; allow both in connect-src. Falls back to
    // 'self' so localhost dev works without extra config.
    const apiOrigin = process.env.NEXT_PUBLIC_API_URL;
    const connectSrc = apiOrigin
      ? ["'self'", apiOrigin, apiOrigin.replace(/^http/, "ws")].join(" ")
      : "'self'";
    return [
      {
        source: "/(.*)",
        headers: [
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
          { key: "X-Frame-Options", value: "SAMEORIGIN" },
          { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
          {
            // Baseline CSP. Next.js needs 'unsafe-inline' for its bootstrap
            // scripts (no nonce in standalone mode); images may come from
            // user uploads / Steam avatars (http/https/blob). Strengthen at
            // the reverse proxy with nonces for a stricter policy.
            key: "Content-Security-Policy",
            value: [
              "default-src 'self'",
              "script-src 'self' 'unsafe-inline'",
              "style-src 'self' 'unsafe-inline'",
              "img-src 'self' data: blob: https: http:",
              "font-src 'self' data:",
              `connect-src ${connectSrc}`,
              "frame-ancestors 'self'",
              "base-uri 'self'",
              "form-action 'self'",
            ].join("; "),
          },
        ],
      },
      {
        source: "/sitemap.xml",
        headers: [
          { key: "Cache-Control", value: "public, s-maxage=3600, stale-while-revalidate=86400" },
        ],
      },
      {
        source: "/robots.txt",
        headers: [
          { key: "Cache-Control", value: "public, s-maxage=3600, stale-while-revalidate=86400" },
        ],
      },
      {
        source: "/rss.xml",
        headers: [
          { key: "Cache-Control", value: "public, s-maxage=300, stale-while-revalidate=600" },
        ],
      },
    ];
  },
};

export default nextConfig;
