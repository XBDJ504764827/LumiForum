# Admin Panel Architecture

**Status:** Accepted for phase 9  
**Scope:** Administrative dashboard, content/user/file/report management, audit logs  
**Access:** `administrator` and `super_administrator` only

## Design goals

- Keep public forum APIs unchanged; admin capabilities live under `/admin/*`.
- Reuse existing domain mutation semantics (topic soft-delete, comment restore, storage-aware upload delete, category manage).
- Enforce RBAC by permission codes, not by hard-coded role names in every handler.
- Enforce role hierarchy by database `roles.priority` under row locks.
- Write an admin audit record in the same transaction as privileged mutations whenever possible.
- Prefer disable/soft-delete for users because authored topics/comments use `ON DELETE RESTRICT`.

## Access model

Frontend `/admin` requires an authenticated user whose role is `administrator` or `super_administrator`.

Backend `/admin/*` requires:

1. Valid access token and active account.
2. Permission `admin.access` (seeded only to administrator and super_administrator).
3. Resource-specific permission for the route group.

Moderator retains existing forum moderation permissions on public content endpoints, but does **not** receive `admin.access` and cannot open the admin panel.

## Permission set

| Code               | Purpose                                           | Roles                                                           |
| ------------------ | ------------------------------------------------- | --------------------------------------------------------------- |
| `admin.access`     | Enter admin panel / call `/admin/*`               | administrator, super_administrator                              |
| `user.manage`      | List/search users, change status, delete (soft)   | administrator, super_administrator                              |
| `user.role.assign` | Assign roles strictly below actor priority        | administrator, super_administrator                              |
| `topic.manage`     | Admin topic list, hide/delete/pin/feature/restore | administrator, super_administrator                              |
| `comment.manage`   | Admin comment list, delete/restore                | administrator, super_administrator                              |
| `category.manage`  | Category CRUD/sort/visibility                     | administrator, super_administrator                              |
| `file.manage`      | Admin upload list/delete/orphan cleanup           | administrator, super_administrator                              |
| `report.manage`    | Review reports                                    | administrator, super_administrator                              |
| `system.manage`    | Dashboard aggregates and admin log inspection     | super_administrator only for logs if needed; both get dashboard |

Existing granular permissions (`topic.pin`, `comment.restore`, etc.) remain for public/moderation flows. Admin routes may accept either the manage permission or the existing granular permission where that keeps reuse simple.

## Module boundaries

```text
routes/admin.rs
    -> services/admin.rs
         -> repositories/admin.rs
         -> existing Category/Topic/Comment/Upload services
         -> AuthorizationService.invalidate
```

- `models/admin.rs`: request/response DTOs and filters.
- `repositories/admin.rs`: admin list queries, role/status transactions, reports, admin_logs, dashboard aggregates.
- `services/admin.rs`: hierarchy policy, audit orchestration, cache invalidation.
- Existing domain services continue to own storage-aware or counter-sensitive mutations.

## Safety rules

- Actor cannot manage users with equal or higher role priority.
- Actor cannot assign a role with priority >= actor priority.
- Actor cannot demote or disable the last remaining `super_administrator`.
- Actor cannot disable/delete self in a way that leaves no super administrator.
- Role/status changes bump `auth_version`, revoke refresh tokens, and invalidate Redis authz cache.
- Hard user delete is replaced by `status = disabled` + optional soft-delete metadata.
- Sensitive UI actions require browser confirm dialogs.

## API surface

- `GET /admin/dashboard`
- `GET|PATCH|DELETE /admin/users...`
- `GET|PATCH|DELETE /admin/topics...`
- `GET|DELETE|POST /admin/comments...`
- `GET|POST|PATCH|DELETE /admin/categories...`
- `GET|DELETE /admin/files...` and orphan cleanup
- `GET|PATCH /admin/reports...`
- `GET /admin/logs`
- Public report creation: `POST /reports`

## Frontend

- App route group `app/admin/*` with dedicated layout, sidebar, and `RequireAdmin` guard.
- TanStack Query keys under `["admin", ...]`.
- Dashboard uses lightweight SVG/CSS charts to avoid new heavy chart dependencies unless needed.
- Management tables share pagination, search, and confirm-action patterns.

## Out of scope

- Full CMS content editor redesign.
- Multipart object reprocessing.
- Multi-admin approval workflows.
- Proxy-aware trusted IP parsing beyond peer `ConnectInfo` and `User-Agent`.
