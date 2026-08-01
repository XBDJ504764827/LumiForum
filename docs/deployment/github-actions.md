# GitHub Actions production deployment

The production workflow is `.github/workflows/deploy-production.yml`. It runs
only for a push to `main` (including a merged pull request) or a manual dispatch
while viewing `main`. Pull request creation, updates to `develop`, and pushes to
other branches do not trigger production deployment.

The deployment job repeats essential source checks before it builds or uploads
anything. Configure `main` branch protection to require the regular `CI`
workflow before merge as the primary quality gate.

## Production layout

Automation matches the existing production paths:

```text
/mnt/1panel/apps/lumiforum/
├── api/
│   ├── .env                         # API runtime configuration; never uploaded
│   ├── lumiforum-api                # active API binary
│   ├── migrate                      # active migration binary
│   └── releases/
│       ├── <commit>-<timestamp>/    # retained candidate API binaries
│       └── rollback-<release>/      # previous active API binaries
└── web/
    ├── .env                         # Web runtime configuration; never uploaded
    ├── current -> releases/<release>
    └── releases/
        └── <commit>-<timestamp>/
            ├── node_modules/
            ├── BUILD-INFO
            └── apps/web/
                ├── server.js
                ├── public/
                └── .next/static/
```

On the first automated release, an existing real `web/current` directory is
moved intact to a rollback release and replaced by the `web/current` symlink.
The active API path deliberately remains `api/lumiforum-api`, matching the
existing service; binaries are replaced atomically after the previous ones are
backed up.

The runner uploads checksummed `lumiforum-api`, `migrate`, and
`lumiforum-web.tar.gz` files to a private temporary directory. The server
verifies checksums, extracts new release directories, runs embedded migrations,
activates API and Web, and restarts both user services. API and Web loopback
health checks must pass. A failed activation or health check restores the
previous application files/link and restarts the previous application version.

Database migrations are not reversed during rollback. Keep migrations backward
compatible with the previous deployed application (expand first, remove old
schema in a later release).

## GitHub production environment

In repository **Settings → Environments**, create an environment named
`production`. Restrict its deployment branches to `main`. Optional required
reviewers can add manual approval; omit reviewers for deployment immediately
after a successful main push.

Add these environment **variables**:

| Variable                | Example                      |
| ----------------------- | ---------------------------- |
| `PROD_DEPLOY_PATH`      | `/mnt/1panel/apps/lumiforum` |
| `PROD_API_PORT`         | `8187`                       |
| `PROD_WEB_PORT`         | `3000`                       |
| `PROD_API_PUBLIC_URL`   | `https://chatapi.cngokz.com` |
| `PROD_SITE_URL`         | `https://chat.cngokz.com`    |
| `PROD_SITE_NAME`        | `LumiForum`                  |
| `PROD_SITE_DESCRIPTION` | `LumiForum community forum`  |

`NEXT_PUBLIC_*` values are embedded into the Web bundle by the workflow. A
change to one of these variables takes effect on the next deployment. During
GitHub's build, server-side page generation uses `PROD_API_PUBLIC_URL` because
the runner cannot access the production server's loopback. At runtime, the
production Web process uses `API_INTERNAL_URL` from `web/.env` (normally
`http://127.0.0.1:8187`).

Add these environment **secrets**:

| Secret                 | Purpose                                           |
| ---------------------- | ------------------------------------------------- |
| `PROD_SSH_HOST`        | Production SSH hostname or IP                     |
| `PROD_SSH_PORT`        | SSH port; an empty value defaults to `22`         |
| `PROD_SSH_USER`        | Dedicated non-root deployment user                |
| `PROD_SSH_PRIVATE_KEY` | Private key dedicated to GitHub production deploy |
| `PROD_SSH_KNOWN_HOSTS` | Pinned SSH host-key line                          |

Do not put PostgreSQL, Redis, JWT, R2, Steam, proxy, or runtime credentials in
GitHub. They stay in `api/.env` and `web/.env` on the server. The workflow never
uploads, copies, or replaces either environment file.

## Create a dedicated deployment key

Generate a new key locally, without reusing a personal administrator key:

```bash
ssh-keygen -t ed25519 -C lumiforum-github-production -f ./lumiforum-production
```

Install `lumiforum-production.pub` in the deployment user's
`~/.ssh/authorized_keys`. Put the complete private key file in
`PROD_SSH_PRIVATE_KEY`, then securely delete the local private copy after the
secret is stored.

Pin the real server key from a trusted machine/network. Replace the host and
port:

```bash
ssh-keyscan -p 22 production.example.com
```

Verify the fingerprint out of band before storing the complete resulting line
in `PROD_SSH_KNOWN_HOSTS`. Do not disable strict host-key checking.

## Server prerequisites

The deployment SSH user must own the application files and its user services:

```bash
DEPLOY_PATH=/mnt/1panel/apps/lumiforum
install -d -m 755 "$DEPLOY_PATH/api/releases" "$DEPLOY_PATH/web/releases"
test -f "$DEPLOY_PATH/api/.env"
test -f "$DEPLOY_PATH/web/.env"
chmod 600 "$DEPLOY_PATH/api/.env" "$DEPLOY_PATH/web/.env"
systemctl --user daemon-reload
systemctl --user enable lumiforum-api lumiforum-web
```

Do not blindly run `chown -R` on a 1Panel tree. Grant the dedicated deployment
user ownership only over this application's `api`, `web/current`, and release
paths after confirming how 1Panel created them.

The user must have linger enabled so user systemd remains available after SSH
logout. An administrator runs this once:

```bash
sudo loginctl enable-linger <deployment-user>
```

The production host needs Node.js 24+, `bash`, `tar`, `sha256sum`, `curl`, and
user-level systemd. It does not need Git, Cargo, pnpm, or application source.
The GitHub runner and server must have compatible Linux architecture, glibc,
and native Node dependencies. The workflow deliberately uses `ubuntu-22.04`;
verify compatibility before the first production run.

The installed service units must use these paths:

```ini
# lumiforum-api.service
WorkingDirectory=/mnt/1panel/apps/lumiforum/api
EnvironmentFile=/mnt/1panel/apps/lumiforum/api/.env
ExecStart=/mnt/1panel/apps/lumiforum/api/lumiforum-api

# lumiforum-web.service
WorkingDirectory=/mnt/1panel/apps/lumiforum/web/current
ExecStart=/absolute/path/to/node --env-file=/mnt/1panel/apps/lumiforum/web/.env /mnt/1panel/apps/lumiforum/web/current/apps/web/server.js
```

Updated templates are in `scripts/deploy/systemd/`. After changing units, run:

```bash
systemctl --user daemon-reload
systemctl --user restart lumiforum-api lumiforum-web
```

The API `.env` is also consumed as a systemd `EnvironmentFile`; use `#` for
comments, not `//`.

## First run and operation

Use **Actions → Deploy production → Run workflow** from `main` for the first
controlled deployment. Before doing so, back up both current application
directories and PostgreSQL. A normal merged pull request then triggers:

```text
merge into main → push event on main → checks/build → upload → migration
→ atomic activation → service restart → health checks
```

The workflow serializes production runs and does not cancel a deployment in
progress. Inspect failures in the Actions run and, on the server, with:

```bash
systemctl --user --no-pager status lumiforum-api lumiforum-web
journalctl --user -u lumiforum-api -u lumiforum-web -n 100 --no-pager
```

Old releases remain under `api/releases/` and `web/releases/` for manual
recovery. Remove them only through a retention procedure that never deletes the
active `web/current` target or the API rollback version still needed.
