# Continuous integration

## CI — `.github/workflows/ci.yml`

Runs on pull requests and pushes to `main` / `develop`.

| Job     | Checks                                                 |
| ------- | ------------------------------------------------------ |
| Web     | install, formatting, typecheck, lint, production build |
| Rust    | rustfmt, check, Clippy (`-D warnings`), tests          |
| Scripts | `bash -n` syntax check of all maintained shell scripts |

Jobs use lockfiles and dependency caches. Permissions are read-only. Concurrency
cancels superseded runs.

## Production deployment — `.github/workflows/deploy-production.yml`

Runs only after a push to `main` (including a merged pull request), or when
manually dispatched from `main`. It validates the source, builds the API and
Next.js standalone artifacts, uploads a checksummed immutable release to a
private R2 deployment bucket, has the production host download it through a
presigned URL, runs migrations, switches release paths, restarts user services,
and performs health checks. Other branch pushes and pull request events do not
trigger it. R2 deployment setup is documented in
[GitHub Actions production deployment](../deployment/github-actions.md).

Repository/environment setup and server prerequisites are documented in
[GitHub Actions production deployment](../deployment/github-actions.md). The
manual procedure remains available in
[Manual production deployment](../deployment/README.md).

## Local parity

```bash
pnpm check
pnpm check:rust
find scripts -type f -name '*.sh' -exec bash -n {} \;
```
