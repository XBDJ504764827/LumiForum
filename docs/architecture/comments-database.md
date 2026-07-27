# Comments Database Design

**Status:** Accepted for phase 4  
**Scope:** Nested comments (max depth 2), soft delete, topic reply stats, RBAC seeds  
**Out of scope:** likes, favorites, notifications, search, realtime

## Goals

- Keep comments independent of future like/notification tables.
- Cap nesting at two levels: root comments and replies to roots only.
- Soft-delete content without hard-removing discussion structure.
- Keep `topics.reply_count` / `last_reply_at` / `last_reply_user_id` consistent under concurrency.
- Avoid N+1 list queries by loading page roots + their children in bulk.
- Extend existing RBAC with explicit comment permissions.

## Entity relationships

```text
topics 1 ──────── * comments
users  1 ──────── * comments
comments 1 ────── * comments (parent_id, depth-limited)
```

## Table: `comments`

| Column        | Type           | Meaning                                |
| ------------- | -------------- | -------------------------------------- |
| `id`          | `uuid` PK      | `gen_random_uuid()`                    |
| `topic_id`    | `uuid` FK      | → `topics(id)` RESTRICT                |
| `author_id`   | `uuid` FK      | → `users(id)` RESTRICT                 |
| `parent_id`   | `uuid` FK null | → `comments(id)` RESTRICT; null = root |
| `content`     | `text`         | Markdown source                        |
| `status`      | `varchar(32)`  | `published` / `deleted`                |
| `like_count`  | `bigint`       | reserved, default 0                    |
| `reply_count` | `bigint`       | direct child replies, default 0        |
| `edited_at`   | `timestamptz`  | null until edited                      |
| `created_at`  | `timestamptz`  | default now()                          |
| `updated_at`  | `timestamptz`  | trigger now(); content edits only      |
| `deleted_at`  | `timestamptz`  | null unless soft-deleted               |

### Checks

- content length 1–20_000
- status published/deleted with soft-delete consistency
- counters ≥ 0
- root comments: `parent_id IS NULL`
- replies: `parent_id IS NOT NULL` and parent must be a root comment in the same topic (enforced in app + DB trigger)

### Indexes

- `(topic_id, created_at ASC, id ASC)` WHERE `parent_id IS NULL AND status = 'published'` for root paging
- `(parent_id, created_at ASC, id ASC)` WHERE `parent_id IS NOT NULL AND status = 'published'` for children
- `(topic_id, created_at DESC)` for admin/moderation scans
- `(author_id, created_at DESC)` for author history later
- partial `deleted_at` index

## Topic stats extension

Add to `topics`:

| Column               | Type           | Meaning                |
| -------------------- | -------------- | ---------------------- |
| `last_reply_user_id` | `uuid` null FK | → `users(id)` SET NULL |

Rules:

- `reply_count` counts published comments of any depth under the topic.
- create/restore increments; soft-delete decrements and recomputes last reply from remaining published comments.
- counter-only topic updates do not rewrite topic content `updated_at` (existing topic trigger already preserves that for non-content fields; `last_reply_*` should be included as non-content fields).

## Soft delete / restore

- Soft delete sets `status='deleted'`, `deleted_at=now()`.
- Public tree omits deleted roots and deleted children.
- If a root is deleted, its children are also soft-deleted (cascade soft-delete) to keep tree coherent.
- Restore re-publishes a single comment if parent/topic still valid; restoring a root does not auto-restore children.

## RBAC seeds

| Code                  | Meaning                      |
| --------------------- | ---------------------------- |
| `comment.create`      | create root comment          |
| `comment.reply`       | reply to a root comment      |
| `comment.update:self` | edit own comment             |
| `comment.update:any`  | edit any comment             |
| `comment.delete:self` | soft-delete own comment      |
| `comment.delete:any`  | soft-delete any comment      |
| `comment.restore`     | restore soft-deleted comment |

Role mapping:

- `user`: create, reply, update:self, delete:self
- `moderator` / `administrator` / `super_administrator`: user set + update:any, delete:any, restore

## List semantics

- Page roots by `created_at ASC, id ASC` (forum-style chronological floors).
- For each page of roots, fetch all published children of those roots in one query.
- Assemble tree in application memory.
- Support `page` / `page_size` now; cursor/infinite scroll can reuse the same repository shape later.

## Rate limit (application layer)

- Redis key `rate:comment:{user_id}` with short TTL window.
- Default: 10 comments/replies per 60 seconds per user.
- Fail closed only on explicit quota; Redis outage logs warning and allows (availability preference for phase 4).

## Deferred

- likes on comments
- infinite nesting
- attachments / reactions
- full-text search
