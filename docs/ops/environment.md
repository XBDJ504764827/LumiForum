# Environment variables

## Rules

1. Commit only `*.example` files.
2. Real values live in `.env` (gitignored).
3. Prefer root `.env` for `docker compose`.
4. Never put secrets in `NEXT_PUBLIC_*` variables.

## Bootstrap

```bash
cp .env.example .env
# optional:
cp apps/api/.env.example apps/api/.env
cp apps/web/.env.example apps/web/.env
```

## Variable map

| Variable                | Consumer      | Purpose                                    |
| ----------------------- | ------------- | ------------------------------------------ |
| `APP_ENV`               | API           | `development` / `production`               |
| `HOST` / `PORT`         | API           | Bind address                               |
| `DATABASE_URL`          | API           | PostgreSQL connection string               |
| `REDIS_URL`             | API           | Redis connection string                    |
| `JWT_SECRET`            | API           | JWT signing secret (≥16 chars outside dev) |
| `CORS_ORIGIN`           | API           | Allowed browser origin                     |
| `RUST_LOG`              | API           | Tracing filter                             |
| `POSTGRES_*`            | Compose       | Official Postgres image bootstrap          |
| `REDIS_PORT`            | Compose       | Host port mapping                          |
| `API_PORT` / `WEB_PORT` | Compose       | Host port mapping                          |
| `NEXT_PUBLIC_API_URL`   | Web (browser) | Public API base URL                        |
| `API_INTERNAL_URL`      | Web (server)  | In-network API base URL                    |
| `S3_*`                  | Future        | S3-compatible object storage               |

## Docker vs local

| Context                        | `DATABASE_URL` host | `REDIS_URL` host | `API_INTERNAL_URL`      |
| ------------------------------ | ------------------- | ---------------- | ----------------------- |
| `docker compose` API container | `postgres`          | `redis`          | n/a                     |
| `docker compose` web container | n/a                 | n/a              | `http://api:8080`       |
| Host `cargo run` / `pnpm dev`  | `localhost`         | `localhost`      | `http://localhost:8080` |

Compose injects service-network URLs for `api` and `web` services; root `.env` defaults target host-side development.
