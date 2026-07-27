# Comments API

**Status:** Implemented in phase 4

## Routes

| Method   | Path                          | Auth               | Purpose                              |
| -------- | ----------------------------- | ------------------ | ------------------------------------ |
| `GET`    | `/topics/{topic_id}/comments` | public             | paged root comments + nested replies |
| `POST`   | `/topics/{topic_id}/comments` | `comment.create`   | create root comment                  |
| `POST`   | `/comments/{id}/reply`        | `comment.reply`    | reply to root comment                |
| `PATCH`  | `/comments/{id}`              | owner/`update:any` | edit comment body                    |
| `DELETE` | `/comments/{id}`              | owner/`delete:any` | soft delete                          |
| `POST`   | `/comments/{id}/restore`      | `comment.restore`  | restore soft-deleted comment         |

## Query

`GET /topics/{topic_id}/comments?page=1&page_size=20`

## Response shapes

List:

```json
{
  "data": {
    "items": [
      {
        "id": "...",
        "content": "...",
        "author": {
          "id": "...",
          "username": "...",
          "nickname": null,
          "avatar": null,
          "role": { "code": "user", "name": "User" }
        },
        "stats": { "likes": 0, "replies": 1 },
        "edited_at": null,
        "created_at": "...",
        "updated_at": "...",
        "replies": []
      }
    ],
    "pagination": { "page": 1, "page_size": 20, "total": 1, "total_pages": 1 }
  }
}
```

Deleted comments are omitted from public trees.
