/**
 * Canonical role codes shared between web and API-bound code paths.
 *
 * These must stay aligned with the `roles` seed in
 * `apps/api/migrations/20260726120000_authentication.up.sql` and with the
 * RBAC constants in `apps/api/src/models/rbac.rs`. Roles are also data-driven
 * (`admin/roles` can adjust permissions), so only *codes* live here — never
 * permission sets.
 *
 * @deprecated Prefer server-driven checks (`user.role.code` compared against
 * this module) over hardcoded role sets inside components; keep the decision
 * of *what a role may do* in the API's RBAC layer.
 */
export const ROLES = {
  USER: "user",
  MODERATOR: "moderator",
  SENIOR_MODERATOR: "senior_moderator",
  ADMINISTRATOR: "administrator",
  SUPER_ADMINISTRATOR: "super_administrator",
} as const;

export type RoleCode = (typeof ROLES)[keyof typeof ROLES];

/** Roles with elevated editorial power over the whole forum (moderation +
 * editing/deleting any content). `senior_moderator` is included: the API
 * seeds it as an optional tier between moderator and administrator
 * (`priority: 25` in `services/moderation.rs`), and the admin UI can assign
 * it dynamically.
 */
export const ELEVATED_ROLE_CODES: ReadonlySet<string> = new Set([
  ROLES.MODERATOR,
  ROLES.SENIOR_MODERATOR,
  ROLES.ADMINISTRATOR,
  ROLES.SUPER_ADMINISTRATOR,
]);

/** Roles that may access the admin console (`admin.access` bound to
 * administrator + super_administrator by the seed data). */
export const ADMIN_ROLE_CODES: ReadonlySet<string> = new Set([
  ROLES.ADMINISTRATOR,
  ROLES.SUPER_ADMINISTRATOR,
]);

export function isElevatedRole(code: string | undefined | null): boolean {
  return code != null && ELEVATED_ROLE_CODES.has(code);
}

export function isAdminRole(code: string | undefined | null): boolean {
  return code != null && ADMIN_ROLE_CODES.has(code);
}
