import { ForumHome } from "@/components/forum/forum-home";
import { JsonLd } from "@/components/seo/json-ld";
import { fetchCategories, fetchTopics } from "@/lib/api/server";
import { websiteJsonLd } from "@/lib/seo/json-ld";
import { homeMetadata } from "@/lib/seo/metadata";

export const metadata = homeMetadata();
export const revalidate = 60;

const PINNED_PARAMS = { sort: "pinned" as const, page: 1, page_size: 5 };
const LATEST_PARAMS = { sort: "latest" as const, page: 1, page_size: 20 };

export default async function HomePage() {
  // Server-side data fetch (ISR): the HTML ships with real content and the
  // client hydrates the identical React Query cache via initialData, so no
  // duplicate request happens in the browser.
  const [categories, pinned, latest] = await Promise.all([
    fetchCategories(),
    fetchTopics(PINNED_PARAMS),
    fetchTopics(LATEST_PARAMS),
  ]);

  return (
    <>
      <JsonLd data={websiteJsonLd()} />
      <ForumHome
        initialCategories={categories ?? []}
        initialPinned={pinned ?? undefined}
        initialLatest={latest ?? undefined}
      />
    </>
  );
}
