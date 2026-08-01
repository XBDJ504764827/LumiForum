# Production security baseline

## Host

1. Keep the OS patched (`unattended-upgrades` on Ubuntu is fine).
2. Firewall allowlist:
   - `22/tcp` (or your custom SSH port) from admin IPs if possible
   - `80/tcp`, `443/tcp` from the world (panel manages these)
   - deny Postgres `5432`, Redis `6379` publicly
3. SSH:
   - `PasswordAuthentication no`
   - `PermitRootLogin no`
   - ed25519 keys only
4. Deploy user: key-only login; no root/sudo needed — everything runs as the
   deploy user itself (user systemd units, no root writes).

## Application bindings

- API binds `127.0.0.1:<API_PORT>` (default 8080); the panel reverse-proxies
  it. If the panel's proxy container cannot reach loopback, bind
  `API_HOST=0.0.0.0` and firewall the port to the panel's network only.
- Web binds `127.0.0.1:<WEB_PORT>` (default 3000); the panel reverse-proxies it.
- Panel should forward real client IPs (X-Forwarded-For) so the API's rate
  limits and peer-IP handling work as intended.

## Secrets

- Store only in host `.env` (mode 600, gitignored) or a secret manager.
- Rotate `JWT_SECRET` only with a planned session invalidation window.
- Prefer R2/S3 over local disk for multi-host readiness.
- `<DEPLOY_PATH>/api/.env` and `<DEPLOY_PATH>/web/.env` hold runtime secrets;
  keep both files mode `600` so other users cannot read them.

## Runtime

- API/Web run as the deploy user with user-systemd hardening
  (`NoNewPrivileges`, `PrivateTmp`, restricted address families).
- Redis binds `127.0.0.1` only.
- PostgreSQL listens on `127.0.0.1`; keep `pg_hba.conf` restricted to the
  loopback host.

## Incident response quick actions

```bash
# freeze traffic edge
systemctl --user stop lumiforum-web lumiforum-api

# inspect
journalctl --user -u lumiforum-api -u lumiforum-web -n 200

# rollback app (replace the release stamps first)
DEPLOY_PATH=/mnt/1panel/apps/lumiforum
OLD_API_STAMP=20260730-02
OLD_WEB_STAMP=20260730-02
install -m 755 "$DEPLOY_PATH/api/releases/$OLD_API_STAMP/lumiforum-api" \
  "$DEPLOY_PATH/api/lumiforum-api"
install -m 755 "$DEPLOY_PATH/api/releases/$OLD_API_STAMP/migrate" \
  "$DEPLOY_PATH/api/migrate"
ln -sfn "releases/$OLD_WEB_STAMP" "$DEPLOY_PATH/web/current"
systemctl --user restart lumiforum-api lumiforum-web
```
