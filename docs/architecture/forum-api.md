# Forum API (Categories + Topics)

**Status:** Implemented

All successful responses use `{ "data": ... }`; errors use the shared
`{ "error": { "code", "message" } }` envelope.

## Public routes

| Method | Path                 | Purpose                                          |
| ------ | -------------------- | ------------------------------------------------ |
| `GET`  | `/categories`        | Visible categories ordered by sort order         |
| `GET`  | `/categories/{slug}` | Visible category detail                          |
| `GET`  | `/topics`            | Published topic list                             |
| `GET`  | `/topics/{slug}`     | Published topic detail and atomic view increment |

Topic list query parameters:

| Parameter   | Values / default                                |
| ----------- | ----------------------------------------------- |
| `category`  | Optional category slug                          |
| `sort`      | `latest` (default), `hot`, `featured`, `pinned` |
| `page`      | 1-based, default 1                              |
| `page_size` | 1–100, default 20                               |

## Authenticated routes

| Method   | Path                      | Permission policy                                    |
| -------- | ------------------------- | ---------------------------------------------------- |
| `POST`   | `/topics`                 | `topic.create`                                       |
| `PATCH`  | `/topics/{id}`            | author + `topic.update:self`, or `topic.update:any`  |
| `DELETE` | `/topics/{id}`            | author + `topic.delete:self`, or `topic.delete:any`  |
| `PATCH`  | `/topics/{id}/moderation` | `topic.pin` and/or `topic.feature` by supplied field |

Delete is a soft delete. Topic slugs remain immutable during normal edits.

## Category management routes

| Method   | Path               | Permission        |
| -------- | ------------------ | ----------------- |
| `POST`   | `/categories`      | `category.manage` |
| `PATCH`  | `/categories/{id}` | `category.manage` |
| `DELETE` | `/categories/{id}` | `category.manage` |

Category delete returns `409 category_not_empty` when any topic references the
category, including soft-deleted topics.

## Boundaries

- Public queries exclude hidden categories and deleted topics in SQL.
- Request sorting is enum-selected; client values never become raw SQL.
- JSON, query, and path extraction failures use the standard validation error.
- Handler middleware authenticates fixed permissions; Service enforces dynamic
  ownership and field-level moderation permissions again.
