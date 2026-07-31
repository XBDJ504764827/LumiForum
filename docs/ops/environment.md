# Environment variables

## Rules

1. Commit only `*.example` files.
2. Real values live in `.env` (gitignored).
3. Never put secrets in `NEXT_PUBLIC_*` variables.
4. Development uses the SSH host LAN address; production URLs are explicit deployment values.

## Development

```bash
cp .env.example .env
pnpm dev:web
```

The committed development template uses `192.168.0.138`, so a browser on the
same network can open `http://192.168.0.138:3000` and call the API at
`http://192.168.0.138:8080`. Both servers bind to `0.0.0.0`.

`pnpm dev` and `pnpm dev:web` load the root `.env` through `dotenv-cli`.

When Next.js is started directly from `apps/web`, it does not load the root
`.env`. In that case the browser client derives the development API URL from
the current page hostname and port `8080`; opening `192.168.0.138:3000`
therefore targets `192.168.0.138:8080`, never the browser machine's localhost.

## Production

Create a private environment file and replace every `CHANGE_ME` value:

```bash
cp .env.production.example .env
```

Use the file only on the build machine. Load it with `dotenv-cli` for the Web
build, then create separate `api.env` and `web.env` files manually on the
production server. Do not upload the combined build-machine file.

`NEXT_PUBLIC_API_URL` and all `NEXT_PUBLIC_SITE_*` values are embedded into the
browser bundle at build time. Changing them requires rebuilding the Web
artifact. `API_INTERNAL_URL`, `HOSTNAME`, and `PORT` are read by the standalone
server at runtime. The API reads all of its configuration from the process
environment and also supports a `.env` in its working directory for direct
manual execution.

Next.js production builds set `NODE_ENV=production`. The API uses `APP_ENV`.
`JWT_SECRET` must always contain at least 32 bytes, and production browser and
CORS URLs must be explicit deployment values.

## Variable map

| Variable                          | Consumer      | Purpose                                |
| --------------------------------- | ------------- | -------------------------------------- |
| `APP_ENV`                         | API           | `development` / `production`           |
| `HOST` / `PORT`                   | API           | Bind address (prod: `127.0.0.1:8080`)  |
| `DATABASE_URL`                    | API           | PostgreSQL connection string           |
| `REDIS_URL`                       | API           | Redis connection string                |
| `JWT_SECRET`                      | API           | JWT signing secret (at least 32 bytes) |
| `JWT_ISSUER`                      | API           | Required JWT issuer claim              |
| `JWT_AUDIENCE`                    | API           | Required JWT audience claim            |
| `ACCESS_TOKEN_TTL_SECONDS`        | API           | Access-token lifetime, 60-3600 seconds |
| `REFRESH_TOKEN_TTL_SECONDS`       | API           | Absolute refresh-family lifetime       |
| `PASSWORD_HASH_CONCURRENCY`       | API           | Maximum concurrent Argon2 operations   |
| `REFRESH_COOKIE_NAME`             | API           | Host-only refresh-cookie name          |
| `REFRESH_COOKIE_SECURE`           | API           | Require HTTPS for refresh cookie       |
| `AUTHORIZATION_CACHE_TTL_SECONDS` | API           | Redis authorization snapshot lifetime  |
| `CORS_ORIGIN`                     | API           | Allowed browser origin (forum domain)  |
| `RUST_LOG`                        | API           | Tracing filter                         |
| `NEXT_PUBLIC_API_URL`             | Web (browser) | Public API base URL (api domain)       |
| `NEXT_PUBLIC_SITE_URL`            | Web           | Canonical site origin (SEO/OG)         |
| `NEXT_PUBLIC_SITE_NAME`           | Web           | Brand name                             |
| `NEXT_PUBLIC_SITE_DESCRIPTION`    | Web           | Default meta description               |
| `API_INTERNAL_URL`                | Web (server)  | Loopback API base URL                  |
| `HOSTNAME` / `PORT`               | Web           | Bind address (prod: `127.0.0.1:3000`)  |
| `STORAGE_*` / `S3_*`              | API           | Object storage                         |
| `STEAM_*`                         | API           | Steam OpenID login (optional)          |
| `WS_*` / `PRESENCE_TTL_SECS`      | API           | Realtime limits                        |

## Production layout

| Context                      | `DATABASE_URL` host | `REDIS_URL` host | `API_INTERNAL_URL`      |
| ---------------------------- | ------------------- | ---------------- | ----------------------- |
| Production (systemd)         | `127.0.0.1`         | `127.0.0.1`      | `http://127.0.0.1:8080` |
| Dev `cargo run` / `pnpm dev` | `192.168.0.62`      | `192.168.0.62`   | `http://localhost:8080` |

The development API runs directly on `192.168.0.138` and connects to
PostgreSQL and Redis on `192.168.0.62`. The database host must allow TCP
connections from `192.168.0.138`. Development Redis requires authentication;
the real password belongs only in the ignored root `.env` using
`redis://:PASSWORD@192.168.0.62:6379`.
