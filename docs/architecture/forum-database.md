# Forum Database Design (Categories + Topics)

**Status:** Accepted for phase 3  
**Database:** PostgreSQL  
**Scope:** Categories, topics, list filters, soft delete, and RBAC permission seeds only  
**Out of scope:** Comments, likes, favorites, notifications, search, admin UI

## Design goals

- Keep forum tables independent from authentication tables.
- Support a large, flat category catalog that can grow without schema changes.
- Support topic lifecycle (publish, soft-delete) without hard-deleting content by default.
- Make list queries for latest / hot / featured / pinned efficient with partial indexes.
- Extend existing RBAC via permission rows; never hard-code role inheritance in SQL.
- Keep slug stable and globally unique for public URLs.
- Reserve reply/like counters for later phases without implementing those features now.

## Entity relationships

```text
users 1 ──────── * topics
categories 1 ──── * topics
```

- A topic always belongs to exactly one category.
- A topic always has exactly one author (`users.id`).
- Category delete is restricted while any topic (including a soft-deleted one) still references it.
- Soft-deleted topics remain in `topics` with `status = 'deleted'` and `deleted_at` set.

## Tables

### `categories`

Flat, orderable forum sections. No parent/child tree in this phase; hierarchy can be added later with a nullable `parent_id` without rewriting topic FKs.

| Column        | Type           | Constraints / meaning                              |
| ------------- | -------------- | -------------------------------------------------- |
| `id`          | `uuid`         | Primary key; `gen_random_uuid()`                   |
| `slug`        | `varchar(64)`  | Required; unique; URL segment                      |
| `name`        | `varchar(100)` | Required; display name                             |
| `description` | `text`         | Nullable short description                         |
| `icon`        | `varchar(64)`  | Nullable icon key or emoji shortcode               |
| `sort_order`  | `integer`      | Required; default `0`; lower first                 |
| `is_visible`  | `boolean`      | Required; default `true`; hidden from public lists |
| `created_at`  | `timestamptz`  | Default `now()`                                    |
| `updated_at`  | `timestamptz`  | Default `now()`; trigger maintained                |

Database checks:

- `slug` matches `^[a-z0-9]+(?:-[a-z0-9]+)*$` and length 2–64.
- `name` trimmed length 1–100.
- `description` null or trimmed length ≤ 2000.
- `icon` null or trimmed length 1–64.
- `sort_order` between `-1_000_000` and `1_000_000`.

Indexes:

- Unique index on `slug`.
- Index on (`is_visible`, `sort_order`, `name`) for public listing.
- Index on `sort_order` for admin listing.

### `topics`

Forum posts. Content is Markdown source stored as `text`. Counters are denormalized for list performance.

| Column          | Type           | Constraints / meaning                        |
| --------------- | -------------- | -------------------------------------------- |
| `id`            | `uuid`         | Primary key; `gen_random_uuid()`             |
| `category_id`   | `uuid`         | FK → `categories(id)` ON DELETE RESTRICT     |
| `author_id`     | `uuid`         | FK → `users(id)` ON DELETE RESTRICT          |
| `title`         | `varchar(200)` | Required                                     |
| `slug`          | `varchar(220)` | Required; unique public URL segment          |
| `content`       | `text`         | Required Markdown source                     |
| `summary`       | `varchar(500)` | Nullable; derived or author-provided excerpt |
| `status`        | `varchar(32)`  | `published` or `deleted` in this phase       |
| `view_count`    | `bigint`       | Default `0`; non-negative                    |
| `reply_count`   | `bigint`       | Default `0`; reserved; non-negative          |
| `like_count`    | `bigint`       | Default `0`; reserved; non-negative          |
| `is_pinned`     | `boolean`      | Default `false`                              |
| `is_featured`   | `boolean`      | Default `false`                              |
| `last_reply_at` | `timestamptz`  | Nullable; reserved for replies phase         |
| `deleted_at`    | `timestamptz`  | Null unless soft-deleted                     |
| `created_at`    | `timestamptz`  | Default `now()`                              |
| `updated_at`    | `timestamptz`  | Default `now()`; trigger maintained          |

Database checks:

- `title` trimmed length 3–200.
- `slug` matches `^[a-z0-9]+(?:-[a-z0-9]+)*$` and length 2–220.
- `content` length 1–100_000 characters.
- `summary` null or trimmed length ≤ 500.
- `status IN ('published', 'deleted')`.
- Soft-delete consistency:
  - `status = 'deleted'` ⇒ `deleted_at IS NOT NULL`
  - `status = 'published'` ⇒ `deleted_at IS NULL`
- Counters ≥ 0.

Indexes:

- Unique index on `slug`.
- Index on (`category_id`, `status`, `created_at DESC`) for category latest lists.
- Index on (`status`, `created_at DESC`) for global latest lists.
- Partial index on (`status`, `view_count DESC`, `created_at DESC`) WHERE `status = 'published'` for hot lists.
- Partial index on (`status`, `created_at DESC`) WHERE `status = 'published' AND is_featured = true` for featured lists.
- Partial index on (`status`, `created_at DESC`) WHERE `status = 'published' AND is_pinned = true` for pinned lists.
- Index on (`author_id`, `created_at DESC`) for author history later.
- Index on `deleted_at` WHERE `deleted_at IS NOT NULL` for cleanup jobs.

## Slug policy

### Categories

- Admin provides or system derives slug from name.
- Normalize: lowercase, strip non-url characters, collapse hyphens.
- Uniqueness is global.

### Topics

- Derived from title on create when not supplied.
- On conflict, append a short random suffix (`-{base36}`) rather than only numeric counters, to reduce race retries under concurrency.
- Slug is immutable after publish unless an admin/moderator explicitly renames later (not required this phase). Editing title does **not** automatically rewrite slug, preserving external links.

## Soft delete

- Public and normal-user queries never return `status = 'deleted'`.
- Soft delete sets:
  - `status = 'deleted'`
  - `deleted_at = now()`
  - `updated_at = now()` (via trigger)
- Restore is out of scope for this phase; schema still allows a future restore by reversing those fields.
- Hard delete is reserved for maintenance tooling, not product APIs.

## List query semantics

| Mode       | Filter                                 | Primary order                                   |
| ---------- | -------------------------------------- | ----------------------------------------------- |
| `latest`   | `status = 'published'`                 | `created_at DESC`, `id DESC`                    |
| `hot`      | `status = 'published'`                 | `view_count DESC`, `created_at DESC`, `id DESC` |
| `featured` | `status = 'published' AND is_featured` | `created_at DESC`, `id DESC`                    |
| `pinned`   | `status = 'published' AND is_pinned`   | `created_at DESC`, `id DESC`                    |

Common rules:

- Optional `category_id` or `category_slug` filter.
- Cursor or page/limit pagination; API layer chooses page/limit first for simplicity.
- Pinned topics may also appear in latest/hot; UI can surface pins separately if desired.
- Hidden categories (`is_visible = false`) are excluded from public category lists and from public topic lists that join categories, unless the actor has category-manage permission.

## RBAC permission seeds (this phase)

New permission rows:

| Code                | Meaning                                                                                     |
| ------------------- | ------------------------------------------------------------------------------------------- |
| `category.read`     | List/read visible categories (authenticated convenience; public guest access is app policy) |
| `category.manage`   | Create/update/delete categories                                                             |
| `topic.read`        | Read published topics                                                                       |
| `topic.create`      | Create topics                                                                               |
| `topic.update:self` | Update own topics                                                                           |
| `topic.update:any`  | Update any topic                                                                            |
| `topic.delete:self` | Soft-delete own topics                                                                      |
| `topic.delete:any`  | Soft-delete any topic                                                                       |
| `topic.pin`         | Toggle pin                                                                                  |
| `topic.feature`     | Toggle featured                                                                             |

Role mapping:

| Role                  | Permissions                                                                             |
| --------------------- | --------------------------------------------------------------------------------------- |
| `user`                | `category.read`, `topic.read`, `topic.create`, `topic.update:self`, `topic.delete:self` |
| `moderator`           | user set + `topic.update:any`, `topic.delete:any`, `topic.pin`, `topic.feature`         |
| `administrator`       | moderator set + `category.manage`                                                       |
| `super_administrator` | all forum permissions                                                                   |

Guest public reads are enforced in application policy for unauthenticated GETs; they do not require a DB role row.

## Integrity and concurrency notes

- Topic create must verify category exists and is usable (visible or actor can manage).
- View count increments are best-effort and may use `UPDATE ... SET view_count = view_count + 1`; exact analytics are not a goal.
- `reply_count` / `like_count` / `last_reply_at` remain zero/null until later phases.
- Foreign keys use `RESTRICT` so user or category removal cannot orphan topics accidentally.

## Deferred schema

Intentionally not created now:

- `comments` / `replies`
- `likes`
- `favorites`
- full-text search vectors
- category tree (`parent_id`)
- topic revision history

## Migration boundary

This document is the schema contract only. Step ② will implement one reversible SQLx migration that:

1. Creates `categories` and `topics`.
2. Adds constraints and indexes above.
3. Seeds the new permissions and role_permissions rows.
4. Leaves authentication tables unchanged.

No migration file is created in this design step.
