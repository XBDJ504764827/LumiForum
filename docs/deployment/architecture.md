# Production deployment architecture

**Status:** Accepted for phase 12  
**Scope:** Single-host production without application containers

## Goals

- Build API and Web artifacts on a compatible Linux build machine.
- Keep source code and build toolchains off the production server.
- Run both applications on loopback under root-managed system-level systemd.
- Let an existing panel, nginx, or Caddy own TLS and reverse proxying.
- Keep releases immutable and make application rollback a symlink switch.

## Topology

```text
Internet
   |
[ TLS reverse proxy :443 ]
   |-- forum.example.com -> 127.0.0.1:3000 (Next.js standalone)
   `-- api.example.com   -> 127.0.0.1:8080 (Axum + WebSocket)

[ systemd ]
   |-- lumiforum-web -> web/current/apps/web/server.js (Node 24+)
   `-- lumiforum-api -> api/lumiforum-api

[ host services ]
   |-- PostgreSQL
   `-- Redis
```

The browser uses the public API URL embedded at Web build time. Next.js
server-side fetching uses `API_INTERNAL_URL=http://127.0.0.1:8080`.

## Runtime artifacts

The API release contains:

```text
api/
├── lumiforum-api
└── migrate
```

SQL migrations are embedded by `sqlx::migrate!`; migration files and source
code are not required at runtime. A GNU binary still depends on the target
CPU architecture, glibc compatibility, and basic runtime libraries.

The Web release is Next.js standalone output plus copied static assets:

```text
web/
├── server.js
├── node_modules/
└── apps/web/
    ├── public/
    └── .next/static/
```

It runs with Node 24+ and does not require pnpm or dependency installation on
the server. Native packages in the output must match the server OS and CPU.

## Filesystem layout

```text
/mnt/1panel/apps/lumiforum/
├── api/
│   ├── .env
│   ├── lumiforum-api
│   ├── migrate
│   └── releases/<stamp>/
└── web/
    ├── .env
    ├── current -> releases/<stamp>
    └── releases/<stamp>/
```

Local uploads must never live under a release directory. S3/R2 is preferred
when external object storage is available. Next.js release files remain
writable by the service user because ISR may update its cache.

## Configuration boundary

- API runtime configuration is supplied by `api/.env`.
- Web runtime configuration is supplied by `web/.env`.
- Public browser variables (`NEXT_PUBLIC_*`) are embedded during `next build`.
  Updating them on the server alone does not alter browser assets.
- Secrets never belong in `NEXT_PUBLIC_*` or release archives.

## Release flow

```text
compatible Linux build host
  -> cargo release binaries + Next standalone
  -> preserve Web static/public layout
  -> tar + SHA-256
  -> upload and verify checksum
  -> extract immutable release
  -> run embedded migrations against candidate API
  -> atomically replace API binaries and switch the Web symlink
  -> restart user services
  -> loopback health checks
```

The `main` branch can deploy this release layout through the GitHub Actions
workflow documented in `docs/deployment/github-actions.md`. The exact manual
commands remain documented in `docs/deployment/README.md` for initialization,
recovery, and controlled manual releases.

## Rollback boundary

Application rollback restores retained API binaries, switches `web/current` to
the retained Web release, and restarts the services. Database migrations are
not automatically reversed, so an older binary must remain compatible with the
migrated schema.

## Security boundaries

- Public firewall ports are limited to SSH and HTTP(S).
- PostgreSQL, Redis, API, and Web bind loopback or a tightly firewalled private
  network.
- Services run through root-managed systemd units with service hardening.
- Runtime env files are readable only by that user.
- The API reverse proxy must support WebSocket upgrade on `/ws`.

## Out of scope

- Multi-host deployment orchestration
- Building on the production server
- Container orchestration
- Multi-host or multi-region operation
- Automatic database downgrade migrations
