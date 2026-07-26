# Authentication Database Design

**Status:** Accepted for authentication phase  
**Database:** PostgreSQL  
**Scope:** Authentication, user profile, refresh-token lifecycle, and RBAC only

## Design goals

- Keep authentication and user-profile data independent from future forum entities.
- Store no plaintext password or refresh token.
- Support short-lived JWT access tokens and revocable, rotating refresh tokens.
- Make role and permission expansion possible without changing application enums or table schemas.
- Make identity uniqueness case-insensitive while preserving the user's chosen casing.
- Retain enough session metadata for revocation, replay detection, and security auditing.

## Entity relationships

```text
roles 1 ──────── * users
  │
  └── * role_permissions * ── permissions

users 1 ──────── * refresh_tokens
                       │
                       └── replacement chain / token family
```

`Guest` is an unauthenticated principal and therefore has no row in `users` or `roles`. Public permissions are assigned by the application policy. Authenticated users have one primary role. If multi-role membership is needed later, a `user_roles` join table can be introduced without changing the permission tables.

## Tables

### `roles`

Roles are data rather than a PostgreSQL enum so that new roles can be added without a database type migration.

| Column        | Type           | Constraints / meaning                                                        |
| ------------- | -------------- | ---------------------------------------------------------------------------- |
| `id`          | `uuid`         | Primary key; generated with `gen_random_uuid()`                              |
| `code`        | `varchar(64)`  | Unique stable machine identifier                                             |
| `name`        | `varchar(100)` | Human-readable name                                                          |
| `description` | `text`         | Nullable description                                                         |
| `priority`    | `smallint`     | Non-negative hierarchy rank; display/default-policy aid only                 |
| `is_system`   | `boolean`      | Protects built-in roles from normal deletion                                 |
| `created_at`  | `timestamptz`  | Defaults to `now()`                                                          |
| `updated_at`  | `timestamptz`  | Defaults to `now()`; maintained explicitly by the application/update trigger |

Initial authenticated roles:

| Code                  | Name                | Priority |
| --------------------- | ------------------- | -------- |
| `user`                | User                | 10       |
| `moderator`           | Moderator           | 20       |
| `administrator`       | Administrator       | 30       |
| `super_administrator` | Super Administrator | 40       |

A higher priority does **not** automatically grant every lower role's permission. Authorization uses explicit permissions. Priority is reserved for hierarchy-sensitive rules, safe role assignment, and UI ordering.

### `permissions`

| Column        | Type           | Constraints / meaning                               |
| ------------- | -------------- | --------------------------------------------------- |
| `id`          | `uuid`         | Primary key; generated with `gen_random_uuid()`     |
| `code`        | `varchar(128)` | Unique stable code such as `user.profile.read:self` |
| `name`        | `varchar(100)` | Human-readable name                                 |
| `description` | `text`         | Nullable description                                |
| `created_at`  | `timestamptz`  | Defaults to `now()`                                 |
| `updated_at`  | `timestamptz`  | Defaults to `now()`                                 |

Permission codes follow `resource.action[:scope]`. Authentication-phase examples include:

- `user.profile.read:self`
- `user.profile.update:self`
- `user.role.assign`
- `user.status.manage`
- `rbac.manage`

Future forum permissions can be added as rows without changing the authentication schema.

### `role_permissions`

| Column          | Type          | Constraints / meaning                               |
| --------------- | ------------- | --------------------------------------------------- |
| `role_id`       | `uuid`        | Foreign key to `roles(id)`, cascade on delete       |
| `permission_id` | `uuid`        | Foreign key to `permissions(id)`, cascade on delete |
| `created_at`    | `timestamptz` | Defaults to `now()`                                 |

The composite primary key is (`role_id`, `permission_id`). An additional index on `permission_id` supports reverse lookups.

### `users`

| Column              | Type           | Constraints / meaning                                                             |
| ------------------- | -------------- | --------------------------------------------------------------------------------- |
| `id`                | `uuid`         | Primary key; generated with `gen_random_uuid()`                                   |
| `username`          | `varchar(32)`  | Required; case-insensitive unique index; application validates allowed characters |
| `email`             | `varchar(254)` | Required; case-insensitive unique index; normalized before persistence            |
| `password_hash`     | `text`         | Required Argon2id PHC string, including salt and parameters                       |
| `avatar`            | `text`         | Nullable URL or object-storage key                                                |
| `nickname`          | `varchar(64)`  | Nullable display name; not unique                                                 |
| `role_id`           | `uuid`         | Required foreign key to `roles(id)`; delete restricted                            |
| `status`            | `varchar(32)`  | `active`, `pending`, `suspended`, or `disabled`                                   |
| `email_verified`    | `boolean`      | Defaults to `false`                                                               |
| `email_verified_at` | `timestamptz`  | Nullable; must agree with `email_verified`                                        |
| `auth_version`      | `integer`      | Defaults to `0`; incremented to invalidate previously issued JWTs                 |
| `created_at`        | `timestamptz`  | Defaults to `now()`                                                               |
| `updated_at`        | `timestamptz`  | Defaults to `now()`                                                               |

Database checks:

- `char_length(username)` is between 3 and 32.
- `nickname` is null or has a trimmed length between 1 and 64.
- `status` belongs to the supported status set.
- `email_verified = false` implies `email_verified_at IS NULL`; verified users require a timestamp.
- `auth_version` is non-negative.

Application validation remains responsible for username syntax, complete email validation, URL validation, and profile input normalization. Password hashes are Argon2id PHC strings; plaintext passwords never enter persistent storage or logs.

Indexes:

- Unique index on `lower(username)`.
- Unique index on `lower(email)`.
- Index on `role_id`.
- Partial index on `status` for non-active users, useful for administration and enforcement jobs.

### `refresh_tokens`

Each row represents one issued opaque refresh token. The browser stores the raw token only in a `Secure`, `HttpOnly` cookie. PostgreSQL stores only its SHA-256 digest.

| Column              | Type           | Constraints / meaning                                                        |
| ------------------- | -------------- | ---------------------------------------------------------------------------- |
| `id`                | `uuid`         | Primary key; generated with `gen_random_uuid()`; public token/JTI identifier |
| `user_id`           | `uuid`         | Foreign key to `users(id)`, cascade on delete                                |
| `family_id`         | `uuid`         | Stable identifier shared by a rotation chain                                 |
| `token_hash`        | `bytea`        | Unique 32-byte SHA-256 digest of the random token secret                     |
| `expires_at`        | `timestamptz`  | Required absolute expiry                                                     |
| `created_at`        | `timestamptz`  | Defaults to `now()`                                                          |
| `last_used_at`      | `timestamptz`  | Nullable successful-use timestamp                                            |
| `revoked_at`        | `timestamptz`  | Nullable revocation timestamp                                                |
| `revocation_reason` | `varchar(64)`  | Nullable machine-readable reason                                             |
| `replaced_by_id`    | `uuid`         | Nullable self-reference to the successor token; delete set null              |
| `created_by_ip`     | `inet`         | Nullable issuance IP address                                                 |
| `user_agent`        | `varchar(512)` | Nullable, length-limited client metadata                                     |

Database checks:

- `octet_length(token_hash) = 32`.
- `expires_at > created_at`.
- `revocation_reason` is absent while the token is active.
- A row cannot replace itself.

Indexes:

- Unique index on `token_hash` for token lookup.
- Index on (`user_id`, `created_at DESC`) for session listing and revoke-all.
- Index on (`family_id`, `created_at`) for rotation-chain replay handling.
- Partial index on `expires_at` where `revoked_at IS NULL` for active-token cleanup.
- Partial unique index on `replaced_by_id` where non-null, preventing two tokens from claiming the same successor.

## Refresh-token lifecycle

1. Login creates a cryptographically random opaque token, stores its SHA-256 digest, and sets the raw value in an HttpOnly cookie.
2. Refresh locks the matching row in a database transaction and verifies that it is unexpired and not revoked.
3. Rotation creates a successor in the same `family_id`, then marks the old row revoked with `replaced_by_id` set.
4. Reuse of an already-rotated token is treated as replay; all active rows in that family are revoked.
5. Logout revokes the current token (or its entire family, depending on endpoint policy) and clears the cookie.
6. Password changes, account suspension, or “log out all devices” revoke every active refresh token for the user.
7. Expired/revoked rows are retained for a bounded audit period and removed by a scheduled cleanup job.

Redis may cache short-lived authorization/session information later, but PostgreSQL is the source of truth for refresh-token validity. This prevents Redis eviction or restart from silently restoring an invalid session.

## Access-token implications

Access tokens are signed JWTs and are not stored in these tables. They contain at minimum `sub` (user UUID), `role`, `auth_version`, `iat`, `exp`, and `jti`. Their lifetime remains short. JWT expiry is enforced cryptographically. Role changes, account suspension, password changes, and global logout increment `users.auth_version`; middleware compares the claim with a short-lived Redis cache backed by PostgreSQL when immediate revocation semantics are required.

## Casing, time, and deletion policy

- Identity comparisons use `lower(username)` and `lower(email)`; original username casing may be retained for display.
- All timestamps use `timestamptz` and are interpreted as UTC.
- Normal account disablement updates `status` instead of deleting the user.
- A hard user deletion cascades refresh tokens, but roles referenced by users are restricted from deletion.
- The application must update `updated_at`; the migration may add one shared trigger to guarantee correctness for direct SQL updates.

## Deferred tables

Email-verification and password-reset token tables are intentionally deferred because no corresponding API is in this phase. When added, they should follow the same opaque-token rule: persist a digest, expiration, and single-use/revocation metadata—never the raw secret.

## Migration boundary

This document defines the schema contract only. The next database implementation step will translate it into an ordered SQLx migration with constraints, indexes, seed rows, and rollback behavior. No migration is created in this design step.
