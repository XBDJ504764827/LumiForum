import type { Metadata } from "next";

import { ForumHome } from "@/components/forum/forum-home";

export const metadata: Metadata = {
  title: "LumiForum",
  description: "社区最新讨论与板块导航",
};

export default function HomePage() {
  return <ForumHome />;
}
