import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  transpilePackages: ["@lumiforum/ui", "@lumiforum/shared", "@lumiforum/types"],
  typedRoutes: true,
};

export default nextConfig;
