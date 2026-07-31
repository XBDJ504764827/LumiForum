# Scripts

## Deploy (`scripts/deploy`)

Only user-level systemd templates are kept in this directory. Production
artifacts are built and uploaded manually as documented in
`docs/deployment/README.md`.

| Path                                     | Purpose                                          |
| ---------------------------------------- | ------------------------------------------------ |
| `systemd/lumiforum-api.service.template` | Runs the compiled Axum API binary                |
| `systemd/lumiforum-web.service.template` | Runs the Next.js standalone server with Node 24+ |
| `systemd/api.env.example`                | Production API runtime environment template      |
| `systemd/web.env.example`                | Production Web runtime environment template      |

The production reverse proxy (1Panel, nginx, or Caddy) forwards public HTTPS
domains to `127.0.0.1:3000` and `127.0.0.1:8080`.

## Backup (`scripts/backup`) — manual tools

| Script                | Purpose                                   |
| --------------------- | ----------------------------------------- |
| `backup-postgres.sh`  | Manual `pg_dump` → gzip into `BACKUP_DIR` |
| `restore-postgres.sh` | Destructive restore helper                |

No automatic backup is scheduled by the scripts; run them manually or via
your own scheduler (e.g. a 1Panel scheduled task).

See `docs/deployment/` for full production procedures.
