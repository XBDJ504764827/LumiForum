# SEO Manual Verification Checklist

Use after deploying phase 11 changes.

## Metadata

1. Open `/` and view source:
   - `<title>` is the site name
   - `meta name="description"` exists
   - `link rel="canonical"` points to the public site origin
   - Open Graph / Twitter tags are present
2. Open a topic page `/topics/{slug}`:
   - title is the topic title
   - description uses summary/content plain text
   - `og:type` is `article`
   - JSON-LD includes `DiscussionForumPosting` and `BreadcrumbList`
3. Open a category page `/categories/{slug}`:
   - title/description match the category
   - breadcrumb JSON-LD is present
4. Open `/login`, `/admin`, `/profile`:
   - `robots` is `noindex`

## Discovery

1. `/robots.txt`
   - allows public routes
   - disallows `/admin`, `/profile`, auth and private pages
   - includes `Sitemap:`
2. `/sitemap.xml`
   - contains home, categories, and published topics
   - topic URLs use `/topics/{slug}`
3. `/rss.xml`
   - valid RSS 2.0
   - latest topics listed
4. `/rss/categories/{slug}`
   - category-specific feed works
   - unknown slug returns 404

## Performance / CWV notes

- Home and category index are statically generated with revalidate windows.
- Topic/category detail metadata and JSON-LD are server-rendered.
- Topic interactive body remains a client island for reactions/comments; LCP should still benefit from server metadata and header shell.
- Markdown images use `loading="lazy"`.
- Response headers include `nosniff`, referrer policy, and cache hints for sitemap/robots/rss.

Suggested checks:

- Lighthouse SEO score on home + topic detail
- Lighthouse Performance on home (LCP / CLS / INP)
- Rich Results Test for topic JSON-LD
- Facebook/Discord link unfurl using the topic URL
