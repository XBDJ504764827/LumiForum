# Turborepo

## Task graph

| Task        | Cache | Notes                                     |
| ----------- | ----- | ----------------------------------------- |
| `build`     | yes   | Depends on dependency packages (`^build`) |
| `typecheck` | yes   | Depends on `^build`                       |
| `lint`      | yes   | Depends on `^build`                       |
| `test`      | yes   | Depends on `^build`                       |
| `dev`       | no    | Persistent; web only via root `pnpm dev`  |
| `clean`     | no    | Local cleanup                             |

## Common commands

```bash
pnpm dev                 # Next.js hot reload
pnpm typecheck           # All packages
pnpm build               # All packages that define build
pnpm build:web           # Web only
pnpm lint                # All packages
pnpm --filter @lumiforum/web typecheck
```

## Filters

Turbo package names match `package.json` `name` fields:

- `@lumiforum/web`
- `@lumiforum/ui`
- `@lumiforum/types`
- `@lumiforum/shared`

Rust API is outside Turbo (Cargo workspace). Orchestrate with Docker Compose or `cargo` directly.
