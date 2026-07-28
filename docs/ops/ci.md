# Continuous integration and release

## CI — `.github/workflows/ci.yml`

Runs on pull requests and pushes to `main` / `develop`.

| Job | Checks |
| --- | --- |
| Web | install, formatting, typecheck, lint, production build |
| Rust | rustfmt, check, Clippy (`-D warnings`), tests |
| Docker | Compose validation (dev + prod), API image build, web image build |

Jobs use lockfiles and dependency caches. Permissions are read-only. Concurrency cancels superseded runs.

## Release — `.github/workflows/release.yml`

Runs on pushes to `main`, version tags `v*.*.*`, and manual dispatch.

1. Build multi-stage production images for API and Web.
2. Push to GHCR:
   - `ghcr.io/<owner>/lumiforum-api:<version>`
   - `ghcr.io/<owner>/lumiforum-web:<version>`
3. Optional SSH deploy to the `production` environment using host checkout + `scripts/deploy/up.sh`.

### Required GitHub configuration

**Variables** (public build-time):

- `NEXT_PUBLIC_API_URL`
- `NEXT_PUBLIC_SITE_URL`
- optional site name/description

**Secrets** (deploy):

- `DEPLOY_HOST`, `DEPLOY_USER`, `DEPLOY_SSH_KEY`
- optional `DEPLOY_PORT`, `DEPLOY_PATH`, `DEPLOY_DOMAIN`

If deploy secrets are absent, images are still published; deploy is skipped/no-op for SSH.

## Local parity

```bash
pnpm check
pnpm check:rust
docker compose --env-file .env.example config --quiet
docker compose -f docker-compose.prod.yml --env-file .env.production.example config
```
