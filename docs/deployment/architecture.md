# Production Deployment Architecture

**Status:** Accepted for phase 12  
**Scope:** Single-host or small-cluster Docker Compose production topology

## Goals

- Ship a repeatable production stack without rewriting application code.
- Keep public traffic on Nginx only; app containers stay on an internal network.
- Support image-based deploys, migration gates, backups, logs, and basic observability.
- Prefer simple rollback via immutable image tags over in-place mutable containers.

## Topology

```text
Internet
   |
   v
[ Nginx :80/:443 ]
   |-- /           --> web:3000   (Next.js standalone)
   |-- /api/       --> api:8080   (Axum, path rewritten)
   |-- /ws         --> api:8080   (WebSocket upgrade)
   |-- /storage/   --> api:8080   (local object storage, optional)
   |
[ internal docker network: lumiforum ]
   |-- api
   |-- web
   |-- postgres
   |-- redis
   |-- backup (cron sidecar / host cron)
   |-- prometheus + grafana + exporters (optional profile)
```

## Design choices

### Compose over Kubernetes (this phase)

Compose is enough for a single VPS / small dedicated host. It keeps ops cost low while still supporting:

- healthchecks
- restart policies
- image pull + recreate deploys
- optional monitoring profile

Kubernetes can replace the orchestrator later without changing app images.

### Edge Nginx

Nginx terminates TLS, applies security headers, rate limits, and routes:

- browser pages to Next.js
- JSON API under `/api/*` to Axum (strip `/api` prefix so existing routes stay `/health`, `/topics`, …)
- WebSocket `/ws` with long-lived upgrade timeouts

This avoids exposing Postgres/Redis/app ports publicly.

### Image tags

| Tag | Meaning |
| --- | --- |
| `sha-<gitsha>` | immutable build artifact |
| `main` / `develop` | moving branch tip |
| `vX.Y.Z` | release |

Production hosts pin to a digest or release tag. Rollback = redeploy previous tag.

### Data plane

- PostgreSQL volume is the source of truth.
- Redis is cache/session/realtime fan-out; AOF enabled in production config.
- Object storage prefers S3/R2 in production; local `/data/uploads` remains supported for small installs.
- API runs SQL migrations on startup (existing behavior) and also has an explicit `migrate` binary for pre-deploy gates.

### Security boundaries

- Only `80/443` (and optionally `22`) on the host firewall.
- Secrets only in host `.env` / secret manager; never in git.
- App containers run as non-root.
- Refresh cookies require HTTPS (`REFRESH_COOKIE_SECURE=true`).

## Deployment flow

```text
CI build + test
  -> push ghcr.io/<owner>/lumiforum-{api,web}:sha-...
  -> deploy job SSHs to host
  -> compose pull
  -> migrate (explicit)
  -> compose up -d --no-deps api web nginx
  -> health checks
```

Blue/green is not required for phase 12. Recreate with healthchecks is the default; for near-zero downtime, run two API replicas behind Nginx upstream later.

## Out of scope

- Multi-region active-active databases
- Full service mesh
- Managed cloud-only IaC (Terraform modules can be added later)
