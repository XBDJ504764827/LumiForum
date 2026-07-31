# Manual production deployment

LumiForum is built on a compatible Linux build machine and uploaded as runtime
artifacts. The production server does not need the repository, Git, Rust,
Cargo, pnpm, TypeScript, or a compiler.

## Runtime requirements

The production server needs:

- Linux and user-level systemd
- Node.js 24+ for the Next.js standalone server
- PostgreSQL 14-17 and Redis reachable by the API
- a TLS reverse proxy such as 1Panel, nginx, or Caddy
- `tar`, `sha256sum`, and `curl`

The API binary is platform-specific. Build for the same CPU architecture as the
server. A GNU target also depends on a compatible glibc; building on a newer
Linux distribution can produce a binary that will not run on an older server.
Use the same/older distribution as the production host, or a separately tested
musl target. Next.js standalone can contain native `sharp` binaries and must
also be built for the production OS and CPU architecture.

## Environment boundary

Use a private `.env` on the build machine for Web build values. Never upload
that combined file to production.

- `NEXT_PUBLIC_API_URL` and `NEXT_PUBLIC_SITE_*` are embedded into browser
  assets. Changing them requires rebuilding the Web artifact.
- `API_INTERNAL_URL`, `HOSTNAME`, and `PORT` are Web runtime values.
- The API reads its configuration from process environment variables. It also
  loads `.env` from its working directory when started directly, but systemd
  should use `EnvironmentFile`.

## 1. Build the artifacts

From the repository root on the Linux build machine:

```bash
cargo build --release --locked -p lumiforum-api \
  --bin lumiforum-api --bin migrate

NODE_ENV=production pnpm exec dotenv -e .env -- pnpm build:web
```

Create one release directory. Replace the example stamp with a version, commit,
or UTC timestamp:

```bash
STAMP=20260731-01
RELEASE_DIR=".release/${STAMP}"
mkdir -p "${RELEASE_DIR}/api" "${RELEASE_DIR}/web/apps/web/.next"

install -m 755 target/release/lumiforum-api "${RELEASE_DIR}/api/"
install -m 755 target/release/migrate "${RELEASE_DIR}/api/"
cp -a apps/web/.next/standalone/. "${RELEASE_DIR}/web/"
cp -a apps/web/.next/static "${RELEASE_DIR}/web/apps/web/.next/static"
cp -a apps/web/public "${RELEASE_DIR}/web/apps/web/public"

printf '%s\n' \
  "git_commit=$(git rev-parse HEAD)" \
  "rustc=$(rustc --version)" \
  "node=$(node --version)" \
  > "${RELEASE_DIR}/BUILD-INFO"

tar -C .release -czf ".release/lumiforum-${STAMP}.tar.gz" "${STAMP}"
(cd .release && sha256sum "lumiforum-${STAMP}.tar.gz" \
  > "lumiforum-${STAMP}.tar.gz.sha256")
```

Do not upload only `apps/web/server.js`. The standalone root `node_modules`,
`apps/web/.next` contents, static files, and `public` directory must keep the
layout above. After extraction, start the bundle from its root with
`node apps/web/server.js`.

Inspect compatibility before uploading:

```bash
file target/release/lumiforum-api target/release/migrate
ldd target/release/lumiforum-api
```

## 2. Initialize the server

Run as the dedicated deployment user. The example path is
`/home/lumiforum/lumiforum`; replace it everywhere with your absolute path.

```bash
DEPLOY_PATH=/home/lumiforum/lumiforum
install -d -m 755 "$DEPLOY_PATH" "$DEPLOY_PATH/releases" "$DEPLOY_PATH/uploads"
install -d -m 700 "$DEPLOY_PATH/env"
install -d -m 755 "$HOME/.config/systemd/user"
```

User services must survive logout. Check the current state:

```bash
loginctl show-user "$USER" -p Linger
```

If it reports `Linger=no`, an administrator must run:

```bash
sudo loginctl enable-linger lumiforum
```

## 3. Create runtime environment files

Create `$DEPLOY_PATH/env/api.env` with mode `600`:

```env
APP_ENV=production
HOST=127.0.0.1
PORT=8080
RUST_LOG=info,tower_http=info,sqlx=warn
DATABASE_URL=postgres://lumiforum:CHANGE_ME@127.0.0.1:5432/lumiforum
REDIS_URL=redis://:CHANGE_ME@127.0.0.1:6379
JWT_SECRET=CHANGE_ME_AT_LEAST_32_BYTES
JWT_ISSUER=lumiforum-api
JWT_AUDIENCE=lumiforum-web
REFRESH_COOKIE_SECURE=true
CORS_ORIGIN=https://forum.example.com
STORAGE_PROVIDER=local
STORAGE_LOCAL_ROOT=/home/lumiforum/lumiforum/uploads
STORAGE_PUBLIC_URL=https://api.example.com/storage
```

Append optional token, S3, realtime, and Steam settings from
`.env.production.example` as needed. `STORAGE_LOCAL_ROOT` must be an absolute
persistent path outside release directories when local storage is used.

Create `$DEPLOY_PATH/env/web.env` with mode `600`:

```env
NODE_ENV=production
HOSTNAME=127.0.0.1
PORT=3000
API_INTERNAL_URL=http://127.0.0.1:8080
NEXT_PUBLIC_SITE_URL=https://forum.example.com
NEXT_PUBLIC_SITE_NAME=LumiForum
NEXT_PUBLIC_SITE_DESCRIPTION="Community forum"
```

The `NEXT_PUBLIC_SITE_*` runtime values keep dynamic rendering and ISR
consistent, but cannot replace the values already embedded during build.

```bash
chmod 600 "$DEPLOY_PATH/env/api.env" "$DEPLOY_PATH/env/web.env"
```

## 4. Install user systemd services

Upload or copy the two templates in `scripts/deploy/systemd/`. Replace
`__DEPLOY_PATH__` with the absolute deployment path in both files. In the Web
unit also replace `__NODE_BIN__` with the output of `command -v node` on the
server, for example `/usr/bin/node`.

Install them as:

```text
~/.config/systemd/user/lumiforum-api.service
~/.config/systemd/user/lumiforum-web.service
```

Then reload systemd:

```bash
systemctl --user daemon-reload
```

## 5. Upload and activate a release

From the build machine:

```bash
scp ".release/lumiforum-${STAMP}.tar.gz" \
    ".release/lumiforum-${STAMP}.tar.gz.sha256" \
    lumiforum@server.example.com:/tmp/
```

On the production server:

```bash
DEPLOY_PATH=/home/lumiforum/lumiforum
STAMP=20260731-01
cd /tmp
sha256sum -c "lumiforum-${STAMP}.tar.gz.sha256"
tar -xzf "lumiforum-${STAMP}.tar.gz" -C "$DEPLOY_PATH/releases"
```

Run the embedded migrations before switching the API symlink. `systemd-run`
loads the same env file without executing it as a shell script:

```bash
systemd-run --user --wait --pipe --collect \
  --property="EnvironmentFile=$DEPLOY_PATH/env/api.env" \
  --property="WorkingDirectory=$DEPLOY_PATH/releases/$STAMP/api" \
  "$DEPLOY_PATH/releases/$STAMP/api/migrate"
```

Activate the release and start services:

```bash
ln -sfn "releases/$STAMP/api" "$DEPLOY_PATH/current-api"
ln -sfn "releases/$STAMP/web" "$DEPLOY_PATH/current-web"
systemctl --user enable --now lumiforum-api lumiforum-web
systemctl --user restart lumiforum-api lumiforum-web
```

Verify locally on the production server:

```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/ready
curl -fsSI http://127.0.0.1:3000/
systemctl --user --no-pager status lumiforum-api lumiforum-web
```

Inspect failures with:

```bash
journalctl --user -u lumiforum-api -u lumiforum-web -n 100 --no-pager
```

## 6. Reverse proxy

Expose only ports 22, 80, and 443 publicly. Configure two HTTPS sites:

| Public URL                  | Upstream                |
| --------------------------- | ----------------------- |
| `https://forum.example.com` | `http://127.0.0.1:3000` |
| `https://api.example.com`   | `http://127.0.0.1:8080` |

Enable WebSocket upgrade for the API `/ws` route. If a panel proxy runs in a
container and cannot reach host loopback, bind the apps to a private host/LAN
address and firewall those ports to the proxy network.

## 7. Manual rollback

Database migrations are not automatically reversed. Confirm that the older
application is compatible with the current schema, then switch symlinks:

```bash
DEPLOY_PATH=/home/lumiforum/lumiforum
OLD_STAMP=20260730-02

test -x "$DEPLOY_PATH/releases/$OLD_STAMP/api/lumiforum-api"
test -f "$DEPLOY_PATH/releases/$OLD_STAMP/web/server.js"
ln -sfn "releases/$OLD_STAMP/api" "$DEPLOY_PATH/current-api"
ln -sfn "releases/$OLD_STAMP/web" "$DEPLOY_PATH/current-web"
systemctl --user restart lumiforum-api lumiforum-web
curl -fsS http://127.0.0.1:8080/ready
curl -fsSI http://127.0.0.1:3000/
```

Keep the release currently referenced by each symlink when deleting old
releases.

## Backups

The repository keeps manual PostgreSQL backup helpers in `scripts/backup/`.
No automatic schedule is installed. Run them on the server or through a 1Panel
scheduled task, copy backups off-site, and regularly test restoration.
