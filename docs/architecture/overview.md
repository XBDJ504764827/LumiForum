# Architecture Overview

## Goals

- Frontend and backend fully separated
- Horizontal scalability of API and web
- Game-agnostic domain model
- AI-friendly monorepo for long-term maintenance

## High-level components

```text
Browser → apps/web (Next.js) → apps/api (Axum) → PostgreSQL
                              ↘ Redis
                              ↘ S3-compatible storage
```

## Package boundaries

| Path              | Role                       | Deployable |
| ----------------- | -------------------------- | ---------- |
| `apps/web`        | Public web UI              | Yes        |
| `apps/api`        | HTTP API                   | Yes        |
| `packages/ui`     | Shared React UI primitives | No         |
| `packages/types`  | Shared TS contracts / DTOs | No         |
| `packages/shared` | Shared TS utilities        | No         |

## Non-goals (Phase 1)

- Forum domain features (auth UI, threads, posts)
- Production multi-region topology
