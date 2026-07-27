# Authentication API

**Status:** Implemented

## Response envelope

Successful responses use:

```json
{
  "data": {}
}
```

Errors use:

```json
{
  "error": {
    "code": "machine_readable_code",
    "message": "safe public message"
  }
}
```

Credential, token, password-hash, and database details are never included in
errors.

## Public routes

| Method | Path             | Body                                         | Result                                  |
| ------ | ---------------- | -------------------------------------------- | --------------------------------------- |
| `POST` | `/auth/register` | username, email, password, optional nickname | access token, user, refresh cookie      |
| `POST` | `/auth/login`    | username/email identifier, password          | access token, user, refresh cookie      |
| `POST` | `/auth/refresh`  | none                                         | rotated access token and refresh cookie |
| `POST` | `/auth/logout`   | none                                         | revokes and removes refresh cookie      |

## Protected routes

| Method  | Path             | Body                                  | Result       |
| ------- | ---------------- | ------------------------------------- | ------------ |
| `GET`   | `/auth/me`       | none                                  | current user |
| `PATCH` | `/users/profile` | optional avatar/nickname patch fields | updated user |

Protected handlers consume an `AuthenticatedPrincipal` inserted only after JWT,
account-state, `auth_version`, role, and permission checks pass.

## Token transport

- Access token: response JSON, then `Authorization: Bearer <token>`.
- Refresh token: host-only `HttpOnly` cookie scoped to `/auth`.
- Cookie `SameSite=Lax`; `Secure` is false only in local development.
- Refresh token values never enter response JSON or application logs.
- Client metadata uses the TCP peer IP and a length-limited User-Agent.
- Refresh without a cookie returns `204`; a supplied invalid token returns `401`.

## Middleware policy

- Guest is the absence of an authenticated user and has no protected permissions.
- Authenticated permissions are loaded from RBAC tables and cached in Redis for
  at most `AUTHORIZATION_CACHE_TTL_SECONDS`.
- Redis failure falls back to PostgreSQL; it never allows a request by default.
- `/auth/me` requires `user.profile.read:self`.
- `/users/profile` requires `user.profile.update:self`.
- Cookie-mutating auth routes require an exact `Origin` match.
- Credentialed CORS permits only the configured explicit origin; wildcard
  origins are rejected during configuration loading.
