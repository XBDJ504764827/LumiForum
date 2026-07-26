# Contributing to LumiForum

## Prerequisites

- Node.js 20 or newer
- pnpm 9.15.0 (managed through Corepack)
- Rust 1.88 or newer with `rustfmt` and `clippy`
- Docker Compose v2 for the full local stack

## Setup

```bash
corepack enable
pnpm install --frozen-lockfile
cp .env.example .env
```

## Quality checks

Run the same checks required by CI before opening a pull request:

```bash
pnpm check
pnpm check:rust
```

Use `pnpm lint:fix`, `pnpm format`, and `cargo fmt --all` to apply safe fixes.

## Boundaries

- Deployable applications belong in `apps/`.
- Reusable TypeScript code belongs in `packages/`.
- Packages must not import from applications.
- Keep API contracts in `packages/types`; do not duplicate DTO shapes in web code.
- Read configuration from environment variables and update the relevant `.env.example`.
- Never commit credentials, generated build output, or local database data.

## Pull requests

- Keep changes focused and document non-obvious architecture decisions in `docs/architecture/`.
- Add tests in proportion to behavioral risk.
- Update operations documentation when commands, variables, or deployment behavior change.
