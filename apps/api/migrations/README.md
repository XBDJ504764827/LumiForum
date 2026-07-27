# API migrations

SQLx migrations live beside the API crate and are applied in version order.

```bash
pnpm db:migrate
```

The API also applies pending migrations after establishing its PostgreSQL
connection. SQLx records checksums in `_sqlx_migrations` and serializes
concurrent migration attempts with PostgreSQL locking.

Migration files are reversible pairs. PostgreSQL is the source of truth for
authentication sessions and RBAC assignments; migration seed data must remain
idempotent within a single migration history.

Do not edit a migration after it has been applied outside local development.
Create a new migration for subsequent schema changes.
