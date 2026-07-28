# LumiForum Docs

Architecture and operations documentation.

## Layout

| Path | Responsibility |
| --- | --- |
| `architecture/` | System design, boundaries, ADRs |
| `deployment/` | Production install, TLS, CI/CD release, rollback |
| `ops/` | Environments, CI notes, runbooks |

## Start here

- [Deployment guide](deployment/README.md)
- [Production architecture](deployment/architecture.md)
- [Environment variables](ops/environment.md)
- [CI / release](ops/ci.md)
- [Security baseline](deployment/security.md)

## Feature architecture

- [Authentication](architecture/authentication-api.md)
- [Forum](architecture/forum-api.md)
- [Uploads](architecture/uploads.md)
- [Admin](architecture/admin.md)
- [Realtime](architecture/realtime.md)
- [SEO](architecture/seo.md)

## Conventions

- Prefer short ADRs for non-obvious decisions.
- Keep docs free of secrets; reference env var names only.
