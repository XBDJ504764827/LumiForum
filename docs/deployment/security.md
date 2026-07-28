# Production security baseline

## Host

1. Keep the OS patched (`unattended-upgrades` on Ubuntu is fine).
2. Firewall allowlist:
   - `22/tcp` (or your custom SSH port) from admin IPs if possible
   - `80/tcp`, `443/tcp` from the world
   - deny Postgres `5432`, Redis `6379`, Grafana `3001` publicly
3. SSH:
   - `PasswordAuthentication no`
   - `PermitRootLogin no`
   - ed25519 keys only
4. Create a non-root deploy user in the `docker` group (or rootless Docker).

## Application edge

Nginx already sets:

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: SAMEORIGIN`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy` locked down
- `Strict-Transport-Security` on HTTPS
- basic request rate limits and connection limits
- `client_max_body_size 25m` aligned with upload limits
- `server_tokens off`

## Secrets

- Store only in host `.env` or a secret manager.
- Rotate `JWT_SECRET` only with a planned session invalidation window.
- Prefer R2/S3 over local disk for multi-host readiness.
- GHCR deploy tokens should be read-only on the server if pulling private images.

## Runtime

- API/Web containers run as uid `10001`.
- Production Redis enables `protected-mode` and stays on the internal network.
- Compose logging caps prevent disk fill from access logs.

## Incident response quick actions

```bash
# freeze traffic edge
docker compose -f docker-compose.prod.yml --env-file .env stop nginx

# inspect
docker compose -f docker-compose.prod.yml --env-file .env logs --tail=200 api

# rollback app images
IMAGE_TAG=sha-<known-good> ./scripts/deploy/rollback.sh
```
