# LumiForum

Modern, long-lived community forum monorepo. Game-agnostic architecture; initially intended for a CS2 community.

## Stack (target)

| Layer   | Technology                                                                    |
| ------- | ----------------------------------------------------------------------------- |
| Web     | Next.js, React, TypeScript, Tailwind CSS, shadcn/ui, TanStack Query, RHF, Zod |
| API     | Rust, Axum, Tokio, SQLx, JWT                                                  |
| Data    | PostgreSQL, Redis, S3-compatible object storage                               |
| Tooling | pnpm, Turborepo, Docker Compose, GitHub Actions                               |

## Repository layout

```text
LumiForum/
├── apps/           # Deployable applications
│   ├── web/        # Next.js frontend
│   └── api/        # Rust Axum backend
├── packages/       # Shared libraries (not deployed alone)
│   ├── ui/         # Design system / shadcn components
│   ├── types/      # Shared TypeScript contracts
│   └── shared/     # Cross-cutting TS utilities
├── docker/         # Dockerfiles & compose overlays
├── scripts/        # Dev / CI helper scripts
├── docs/           # Architecture & ops docs
└── .github/        # CI/CD workflows
```

## Status

Application phases 1–11 are implemented (forum product + SEO). Phase 12 adds
production Docker Compose, Nginx/TLS, GHCR release, backups, and monitoring.

## Quick start (development)

```bash
cp .env.example .env
docker compose up --build
```

Services become available at:

- Web: <http://192.168.0.138:3000>
- API health: <http://192.168.0.138:8080/health>
- API readiness: <http://192.168.0.138:8080/ready>

For host-side development, start PostgreSQL and Redis first, then run:

```bash
pnpm install --frozen-lockfile
pnpm dev:web
cargo run -p lumiforum-api
```

## Production

See [`docs/deployment/README.md`](docs/deployment/README.md).

```bash
cp .env.production.example .env
# edit DOMAIN, secrets, public URLs
./scripts/deploy/up.sh
./scripts/deploy/init-certs.sh
```

## Quality checks

```bash
pnpm check
pnpm build:web
pnpm check:rust
```

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`docs/`](docs/) for repository
conventions and operations notes.

## License

MIT
