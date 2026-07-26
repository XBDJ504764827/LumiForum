# Environment variables

## Rules

1. Commit only `*.example` files.
2. Real values live in `.env` (gitignored).
3. Prefer root `.env` for `docker compose` and host-side development.
4. Never put secrets in `NEXT_PUBLIC_*` variables.
5. Development uses the SSH host LAN address; production URLs are explicit deployment values.

## Development

```bash
cp .env.example .env
pnpm dev:web
```

The committed development template uses `192.168.0.138`, so a browser on the
same network can open `http://192.168.0.138:3000` and call the API at
`http://192.168.0.138:8080`. Both servers bind to `0.0.0.0`.

`pnpm dev` and `pnpm dev:web` load the root `.env` through `dotenv-cli`.
Docker Compose loads the same file automatically.

## Production

Create a private environment file and replace every `CHANGE_ME` value:

```bash
cp .env.production.example .env
```

Next.js production builds set `NODE_ENV=production`. The API uses `APP_ENV`
and rejects a short JWT secret outside development. Production browser and
CORS URLs must be explicit because they are deployment-specific.

`NEXT_PUBLIC_API_URL` is embedded in browser assets at build time. Pass it to
the production image build:

```bash
docker build \
  --build-arg NEXT_PUBLIC_API_URL=https://api.example.com \
  -f docker/web/Dockerfile \
  -t lumiforum-web .
```

## Variable map

| Variable                | Consumer      | Purpose                                     |
| ----------------------- | ------------- | ------------------------------------------- |
| `APP_ENV`               | API           | `development` / `production`                |
| `NODE_ENV`              | Web           | Next.js runtime mode                        |
| `DEV_HOST`              | Tooling       | LAN address for remote development          |
| `HOST` / `PORT`         | API           | Bind address                                |
| `DATABASE_URL`          | API           | PostgreSQL connection string                |
| `REDIS_URL`             | API           | Redis connection string                     |
| `JWT_SECRET`            | API           | JWT signing secret (>=16 chars outside dev) |
| `CORS_ORIGIN`           | API           | Allowed browser origin                      |
| `RUST_LOG`              | API           | Tracing filter                              |
| `POSTGRES_*`            | Compose       | Official Postgres image bootstrap           |
| `REDIS_PORT`            | Compose       | Host port mapping                           |
| `API_PORT` / `WEB_PORT` | Compose       | Host port mapping                           |
| `NEXT_PUBLIC_API_URL`   | Web (browser) | Public API base URL                         |
| `API_INTERNAL_URL`      | Web (server)  | In-network API base URL                     |
| `S3_*`                  | Future        | S3-compatible object storage                |

## Docker vs local

| Context                        | `DATABASE_URL` host | `REDIS_URL` host | `API_INTERNAL_URL`      |
| ------------------------------ | ------------------- | ---------------- | ----------------------- |
| `docker compose` API container | `postgres`          | `redis`          | n/a                     |
| `docker compose` web container | n/a                 | n/a              | `http://api:8080`       |
| Host `cargo run` / `pnpm dev`  | `localhost`         | `localhost`      | `http://localhost:8080` |

Compose overrides data-service URLs with internal DNS names while public
browser URLs continue to use the development or production host.
