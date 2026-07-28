# Deployment guide

Production deployment for LumiForum using Docker Compose, Nginx, Let’s Encrypt, GHCR images, backups, and optional monitoring.

## Architecture

See [architecture.md](./architecture.md).

## Server requirements

| Resource | Minimum | Recommended |
| --- | --- | --- |
| CPU | 2 vCPU | 4 vCPU |
| RAM | 4 GB | 8 GB |
| Disk | 40 GB SSD | 80 GB+ SSD |
| OS | Ubuntu 22.04/24.04 LTS | same |
| Network | public IPv4 + DNS A/AAAA for `DOMAIN` | |

Install:

- Docker Engine 24+
- Docker Compose plugin v2
- `curl`, `git`, `uFW` (or equivalent firewall)

## 1. Bootstrap the host

```bash
sudo mkdir -p /opt/lumiforum
sudo chown "$USER:$USER" /opt/lumiforum
cd /opt/lumiforum
git clone <your-fork-or-repo> .
cp .env.production.example .env
chmod 600 .env
```

Edit `.env`:

- `DOMAIN`, `CERTBOT_EMAIL`
- `POSTGRES_PASSWORD`, `JWT_SECRET` (≥ 32 chars)
- `CORS_ORIGIN` / `NEXT_PUBLIC_SITE_URL` / `NEXT_PUBLIC_API_URL`
- storage credentials if using R2/S3

Firewall (example):

```bash
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow OpenSSH
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
```

SSH hardening tips: key-only auth, disable root password login, optional `Fail2ban`.

## 2. First start (HTTP bootstrap)

```bash
chmod +x scripts/deploy/*.sh scripts/backup/*.sh
./scripts/deploy/up.sh
./scripts/deploy/smoke.sh http://127.0.0.1
```

On first boot without certificates, Nginx uses the HTTP bootstrap template so ACME can succeed.

## 3. HTTPS

Point DNS `DOMAIN` to the server, then:

```bash
./scripts/deploy/init-certs.sh
./scripts/deploy/smoke.sh "https://$DOMAIN"
```

Renew daily via cron:

```cron
0 4 * * * cd /opt/lumiforum && docker compose -f docker-compose.prod.yml --env-file .env run --rm --profile certs certbot renew && docker compose -f docker-compose.prod.yml --env-file .env exec nginx nginx -s reload
```

## 4. CI/CD

### GitHub configuration

Repository **Variables** (build-time public URLs):

- `NEXT_PUBLIC_API_URL` e.g. `https://forum.example.com/api`
- `NEXT_PUBLIC_SITE_URL` e.g. `https://forum.example.com`
- optional `NEXT_PUBLIC_SITE_NAME`, `NEXT_PUBLIC_SITE_DESCRIPTION`

Repository / environment **Secrets** for deploy:

- `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`
- optional `DEPLOY_PORT`, `DEPLOY_PATH` (default `/opt/lumiforum`)
- optional `DEPLOY_DOMAIN` for HTTPS smoke tests

Create a GitHub Environment named `production` if you want approval gates.

### Pipelines

| Workflow | Trigger | Purpose |
| --- | --- | --- |
| `ci.yml` | PR / push | lint, test, compose validate, image build (no push) |
| `release.yml` | push `main` / tags `v*` | build+push GHCR, optional SSH deploy |

Images:

- `ghcr.io/<owner>/lumiforum-api:<version>`
- `ghcr.io/<owner>/lumiforum-web:<version>`

`<version>` is `sha-<fullsha>` on branch pushes or the semver tag on releases.

### Manual deploy

```bash
export IMAGE_API=ghcr.io/<owner>/lumiforum-api:sha-...
export IMAGE_WEB=ghcr.io/<owner>/lumiforum-web:sha-...
# write into .env or:
ENV_FILE=.env ./scripts/deploy/up.sh
```

### Rollback

```bash
IMAGE_TAG=sha-<previous> ./scripts/deploy/rollback.sh
# or
IMAGE_API=... IMAGE_WEB=... ./scripts/deploy/rollback.sh
```

## 5. Database migrations

`./scripts/deploy/up.sh` runs `/usr/local/bin/lumiforum-migrate` before recreating API/Web.

The API process also applies migrations on boot as a safety net. Prefer the explicit migrate step in deploys so schema changes fail before traffic cutover.

## 6. Backups

The `backup` service dumps Postgres daily (default 03:00 UTC) into the `backup_data` volume:

```text
/backups/lumiforum-<db>-<timestamp>.sql.gz
```

Retention: `BACKUP_RETENTION_DAYS` (default 14).

Restore (destructive):

```bash
# copy dump out if needed
docker compose -f docker-compose.prod.yml --env-file .env cp backup:/backups/<file> ./
POSTGRES_USER=... POSTGRES_DB=lumiforum ./scripts/backup/restore-postgres.sh ./<file>
```

Schedule a host-level copy of `/var/lib/docker/volumes/..._backup_data` or `docker run --rm -v ...` sync to offsite object storage.

## 7. Logs

Compose uses `json-file` rotation (`20m` × 7 files) per service.

```bash
docker compose -f docker-compose.prod.yml --env-file .env logs -f api
docker compose -f docker-compose.prod.yml --env-file .env logs -f nginx
```

Nginx logs also live in the `nginx_logs` volume.

## 8. Monitoring (optional)

```bash
docker compose -f docker-compose.prod.yml --env-file .env --profile monitoring up -d
```

- Prometheus: internal only
- Grafana: `127.0.0.1:3001` (SSH tunnel recommended)
- Exporters: Postgres + Redis

Application `/metrics` can be added later without changing the scrape layout.

## 9. Security checklist

- [ ] `.env` mode `600`, not in git
- [ ] UFW/security group: only 22/80/443
- [ ] `JWT_SECRET` long random value
- [ ] `REFRESH_COOKIE_SECURE=true`
- [ ] S3 credentials rotated and least-privilege
- [ ] GHCR packages private unless intentional
- [ ] Deploy SSH key is ed25519, limited to deploy user
- [ ] Regular backup restore drill

## 10. Routing map

| Public path | Upstream |
| --- | --- |
| `/` | `web:3000` |
| `/api/*` | `api:8080/*` (prefix stripped) |
| `/ws` | `api:8080/ws` |
| `/storage/*` | `api:8080/storage/*` |
| `/health`, `/ready` | `api` |

Browser `NEXT_PUBLIC_API_URL` should be `https://<domain>/api` when using this edge layout.

## 11. Troubleshooting

| Symptom | Check |
| --- | --- |
| Nginx restart loop | cert missing → bootstrap path; `docker compose logs nginx` |
| API unhealthy | `DATABASE_URL` / Redis; `logs api` |
| Web 500 on SEO routes | `API_INTERNAL_URL=http://api:8080` |
| WS fails | Nginx `/ws` upgrade headers; browser uses `wss://` |
| Migrate fails | fix SQL before traffic switch; do not skip |

## Related docs

- [architecture.md](./architecture.md)
- [../ops/environment.md](../ops/environment.md)
- [../ops/ci.md](../ops/ci.md)
