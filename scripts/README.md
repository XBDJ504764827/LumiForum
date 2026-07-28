# Scripts

## Deploy (`scripts/deploy`)

| Script | Purpose |
| --- | --- |
| `up.sh` | Pull/start prod stack, migrate, health-check |
| `init-certs.sh` | First Let's Encrypt certificate |
| `rollback.sh` | Pin previous image tags and recreate app |
| `smoke.sh` | Post-deploy HTTP checks |

## Backup (`scripts/backup`)

| Script | Purpose |
| --- | --- |
| `backup-postgres.sh` | `pg_dump` → gzip in `/backups` |
| `backup-entrypoint.sh` | Daily loop used by the backup service |
| `restore-postgres.sh` | Destructive restore helper |

See `docs/deployment/` for full production procedures.
