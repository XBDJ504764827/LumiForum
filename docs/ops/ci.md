# Continuous integration

## CI — `.github/workflows/ci.yml`

Runs on pull requests and pushes to `main` / `develop`.

| Job     | Checks                                                 |
| ------- | ------------------------------------------------------ |
| Web     | install, formatting, typecheck, lint, production build |
| Rust    | rustfmt, check, Clippy (`-D warnings`), tests          |
| Scripts | `bash -n` syntax check of maintained shell scripts     |

Jobs use lockfiles and dependency caches. Permissions are read-only. Concurrency
cancels superseded runs.

There is no image registry or automatic deploy workflow. Production artifacts
are built and uploaded with the manual commands in
[docs/deployment/README.md](../deployment/README.md).

## Local parity

```bash
pnpm check
pnpm check:rust
for f in scripts/backup/*.sh; do bash -n "$f"; done
```
