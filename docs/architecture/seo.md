# SEO Architecture

**Status:** Accepted for phase 11  
**Scope:** Public page metadata, sitemap, robots, JSON-LD, RSS, caching

## Goals

- Make public forum surfaces crawlable without requiring client JS.
- Keep private routes (`/admin`, `/profile`, auth, notifications, favorites) out of indexes.
- Preserve existing public URL shapes (`/`, `/categories`, `/categories/{slug}`, `/topics/{slug}`).
- Prefer server-side data for metadata and feeds so crawlers see complete HTML.

## Public vs private

| Surface                                        | Index | Notes                          |
| ---------------------------------------------- | ----- | ------------------------------ |
| Home, categories, topic detail, search landing | yes   | Canonical + OG                 |
| Login/register                                 | no    | `noindex`                      |
| Profile, favorites, notifications              | no    | Authenticated UX               |
| Admin                                          | no    | Blocked in robots and metadata |
| Topic edit / new topic                         | no    | Authoring flows                |

## URL policy

Existing routes remain canonical:

- `/`
- `/categories`
- `/categories/{slug}`
- `/topics/{slug}`
- `/search` (indexable landing only; deep query URLs use `noindex,follow` to reduce thin/duplicate pages)

Alias paths such as `/topic/{slug}` or `/user/{username}` are **not** introduced in this phase to avoid duplicate content. If aliases are added later, they must 301 to the canonical routes above.

## Metadata system

Shared helpers under `apps/web/src/lib/seo/`:

- site config (`SITE_URL`, site name, default description)
- title templates
- canonical builder
- Open Graph / Twitter card builders
- description sanitizer (strip markdown noise, clamp length)

Page `generateMetadata` loads public API data on the server via `API_INTERNAL_URL` when available.

## Structured data

JSON-LD injected as `<script type="application/ld+json">` from server components:

- `WebSite` on home
- `CollectionPage` / `BreadcrumbList` on category pages
- `DiscussionForumPosting` + `BreadcrumbList` on topic pages
- interaction statistics from topic counters when present

## Discovery

- `/robots.txt` — allow public routes, disallow private ones, point to sitemap
- `/sitemap.xml` — index of paginated child sitemaps
- `/sitemap/topics/{page}.xml`, `/sitemap/categories.xml` — chunked generation
- `/rss.xml` — latest topics
- `/rss/categories/{slug}.xml` — per-category feeds

Sitemap generation pages through the public topics API and never loads the full corpus into memory.

## Caching

- Metadata/public fetches use short revalidate windows (home/category ~60s, topic ~30–120s).
- Sitemap/RSS use longer revalidate windows and explicit cache tags where useful.
- Private pages remain dynamic and uncached for SEO purposes.

## Out of scope

- Full server-rendered topic body replacement of the interactive client view (kept for reactions/comments UX)
- OG image generation service
- Changing production path prefixes
