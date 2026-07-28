import { ForumHome } from "@/components/forum/forum-home";
import { JsonLd } from "@/components/seo/json-ld";
import { websiteJsonLd } from "@/lib/seo/json-ld";
import { homeMetadata } from "@/lib/seo/metadata";

export const metadata = homeMetadata();
export const revalidate = 60;

export default function HomePage() {
  return (
    <>
      <JsonLd data={websiteJsonLd()} />
      <ForumHome />
    </>
  );
}
