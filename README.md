# LumiForum

Modern, long-lived community forum monorepo. Game-agnostic architecture; initially intended for a CS2 community.

## Stack

| Layer   | Technology                                                                    |
| ------- | ----------------------------------------------------------------------------- |
| Web     | Next.js, React, TypeScript, Tailwind CSS, shadcn/ui, TanStack Query, RHF, Zod |
| API     | Rust, Axum, Tokio, SQLx, JWT                                                  |
| Data    | PostgreSQL, Redis, S3-compatible object storage                               |
| Tooling | pnpm, Turborepo, GitHub Actions, systemd                                      |

## Repository layout

```text
LumiForum/
├── apps/           # Deployable applications
│   ├── web/        # Next.js frontend (standalone build)
│   └── api/        # Rust Axum backend (release binary)
├── packages/       # Shared libraries (not deployed alone)
│   ├── ui/         # Design system / shadcn components
│   ├── types/      # Shared TypeScript contracts
│   └── shared/     # Cross-cutting TS utilities
├── scripts/        # Deploy / backup / helper scripts
├── docs/           # Architecture & ops docs
└── .github/        # CI workflows
```

## Status

Application phases 1–11 are implemented (forum product + SEO). Phase 12 adds
production deployment: locally-built binaries, host PostgreSQL/Redis, Nginx/TLS,
systemd, and backups.

## Quick start (development)

For host-side development, start PostgreSQL and Redis first, then run:

```bash
cp .env.example .env
pnpm install --frozen-lockfile
pnpm dev:web
cargo run -p lumiforum-api
```

Services become available at:

- Web: <http://192.168.0.138:3000>
- API health: <http://192.168.0.138:8080/health>
- API readiness: <http://192.168.0.138:8080/ready>

## Production

See [`docs/deployment/README.md`](docs/deployment/README.md).

Build production artifacts on a compatible Linux machine, then upload them to
the server. The server runs the API binary and Next.js standalone bundle with
user-level systemd services; it does not need Rust, pnpm, or the repository.

```bash
cargo build --release --locked -p lumiforum-api \
  --bin lumiforum-api --bin migrate
NODE_ENV=production pnpm exec dotenv -e .env -- pnpm build:web
```

See the deployment guide for the required artifact layout, environment files,
manual upload, migration, service installation, and rollback commands.

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
