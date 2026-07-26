# Continuous integration

`.github/workflows/ci.yml` runs three independent jobs on pull requests and
pushes to `main` or `develop`.

| Job    | Checks                                                 |
| ------ | ------------------------------------------------------ |
| Web    | install, formatting, typecheck, lint, production build |
| Rust   | rustfmt, check, Clippy with warnings denied, tests     |
| Docker | Compose validation, API image, web image               |

Jobs use lockfiles and dependency caches. Workflow permissions are read-only,
and concurrency cancellation prevents superseded commits from consuming CI
capacity.

Dependabot groups routine updates by ecosystem to keep maintenance noise low.
