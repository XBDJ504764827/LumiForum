# Steam Authentication Architecture

**Status:** Accepted  
**Scope:** Steam OpenID 2.0 login, bind/unbind, and profile sync alongside password authentication

This design extends the session model in [Web authentication architecture](./authentication-web.md). Access tokens remain in browser memory and refresh tokens remain in the API's host-only HttpOnly cookie.

## Goals

- Add Steam as a first-class login provider without breaking username/email login.
- Reuse the existing access/refresh session lifecycle and refresh-token rotation.
- Support auto-provisioning, bind, unbind, and profile sync through the Steam Web API.
- Keep OpenID verification first-party without a third-party Steam SDK.
- Never expose an access token, refresh token, or authorization credential in a redirect URL.

## Endpoint contract

| Endpoint                    | Authentication                  | Success result                                                                          |
| --------------------------- | ------------------------------- | --------------------------------------------------------------------------------------- |
| `GET /auth/steam/login`     | Anonymous                       | `302` redirect to the Steam OpenID authorization URL                                    |
| `GET /auth/steam/callback`  | OpenID callback                 | Set/rotate the refresh cookie when logging in, then redirect to the web completion page |
| `POST /auth/steam/bind`     | Bearer access token             | JSON `data.authorization_url`; the browser navigates to this URL                        |
| `DELETE /auth/steam/unbind` | Bearer access token + JSON body | Updated `User`                                                                          |
| `POST /auth/steam/sync`     | Bearer access token             | Updated `User`                                                                          |

`POST /auth/steam/bind` is intentionally separate from the browser redirect. The browser cannot attach the in-memory Bearer token to a top-level navigation, so the authenticated POST creates one-time bind state before returning the public Steam authorization URL.

The unbind request body is:

```json
{
  "password": "current password"
}
```

## Flows

### Login

```text
Browser -> GET /auth/steam/login
API -> create one-time login state (CSRF protection, short TTL)
API -> 302 Steam OpenID
Steam -> GET /auth/steam/callback?...OpenID assertion...
API -> validate state and consume it exactly once
API -> verify the assertion with Steam check_authentication
API -> parse SteamID64 and fetch GetPlayerSummaries
API -> find the user by steam_id or provision a Steam-only user
API -> issue the normal session and set/rotate the HttpOnly refresh cookie
API -> 302 {STEAM_WEB_ORIGIN}/auth/steam/complete
Completion page -> POST /auth/refresh with credentials included
Completion page -> GET /auth/me with the new in-memory access token
Completion page -> replace navigation to /
```

The callback never includes an access token or refresh token in the path, query, or fragment. The completion page defensively removes an unexpected URL fragment without parsing or storing it.

### Bind

```text
Authenticated browser -> POST /auth/steam/bind with Bearer access token
API -> create one-time bind state associated with the current user
API -> 200 { data: { authorization_url } }
Browser -> navigate to authorization_url
Steam -> GET /auth/steam/callback?...OpenID assertion...
API -> validate and consume bind state, then verify the assertion
API -> bind SteamID64 unless it belongs to another user
API -> 302 {STEAM_WEB_ORIGIN}/auth/steam/complete?mode=bind
Completion page -> restore the existing session through /auth/refresh and /auth/me
Completion page -> replace navigation to /profile
```

A bind callback does not accept a user ID from the browser. The target user comes only from server-side one-time state created by the authenticated bind request.

### Unbind

```text
Authenticated browser -> DELETE /auth/steam/unbind { password }
API -> verify the current password
API -> reject when has_password is false, because Steam is the sole login method
API -> clear Steam identity and profile fields
API -> return the updated User
Browser -> replace the AuthProvider user with the returned User
```

The profile UI does not offer unbind when `User.has_password` is false and tells the user to set a password first. The API remains the security boundary and must enforce the same rule.

### Profile sync

```text
Authenticated browser -> POST /auth/steam/sync
API -> fetch the bound Steam profile
API -> update cached Steam profile fields
API -> return the updated User
Browser -> replace the AuthProvider user with the returned User
```

## Completion redirect contract

The API redirects only to the configured frontend origin and completion path:

- Login success: `/auth/steam/complete`
- Bind success: `/auth/steam/complete?mode=bind`
- Login failure: `/auth/steam/complete?error=<code>`
- Bind failure: `/auth/steam/complete?mode=bind&error=<code>`

Only `mode` and a stable, non-sensitive `error` code belong in the query. OpenID assertions, one-time state, access tokens, refresh tokens, and internal error details must not be copied into the frontend redirect.

Stable Steam error codes are:

| Code                             | Meaning                                                                                 |
| -------------------------------- | --------------------------------------------------------------------------------------- |
| `steam_access_denied`            | The user cancelled or denied Steam authorization                                        |
| `steam_invalid_state`            | State is missing, expired, already consumed, or does not match the flow                 |
| `steam_auth_failed`              | OpenID verification, Steam identity validation, or required Steam profile lookup failed |
| `steam_account_conflict`         | The SteamID64 is already bound to a different user                                      |
| `steam_not_bound`                | Sync or unbind was requested without a bound Steam account                              |
| `steam_unbind_requires_password` | Unbind would remove the user's only login method                                        |
| `invalid_credentials`            | The password supplied for unbind is incorrect                                           |
| `steam_unavailable`              | A required Steam service is temporarily unavailable                                     |

Unknown completion error codes receive a generic message. Logs may retain diagnostic context, but redirects and API responses must not expose OpenID assertions, Steam API keys, tokens, or raw upstream responses.

## Data model and API DTO

`User` exposes these authentication/profile fields to the frontend:

| Field                 | Notes                                                                 |
| --------------------- | --------------------------------------------------------------------- |
| `steam_id`            | Nullable unique SteamID64 string                                      |
| `steam_persona_name`  | Last synchronized persona name                                        |
| `steam_avatar`        | Small Steam avatar URL                                                |
| `steam_avatar_medium` | Medium Steam avatar URL                                               |
| `steam_avatar_full`   | Full Steam avatar URL                                                 |
| `steam_profile_url`   | Public Steam community profile URL                                    |
| `has_password`        | Whether password authentication is available; controls safe unbind UI |
| `steam_country_code`  | Optional normalized two-letter Steam country code                     |

Steam authorization uses a dedicated response DTO containing only `authorization_url`. Bind state and Steam credentials are never included in a frontend DTO.

Steam-only accounts are provisioned automatically without a registration form. They use SteamID64 as the forum `username`, the current Steam persona name as `nickname`, the medium Steam avatar (falling back to the small avatar) as the forum `avatar`, a nullable password hash, and an internal unique placeholder email. On each Steam login these forum identity fields are refreshed for Steam-only accounts. Binding Steam to an existing password account does not overwrite that account's username, nickname, or forum avatar. Placeholder email values are implementation details and must not be treated as Steam-verified email data.

## Security constraints

- Require exact configured HTTPS `return_to` and `realm` values outside local development.
- Accept only `openid.mode=id_res` and always verify the assertion through Steam `check_authentication` over TLS.
- Require the claimed identity to exactly match `https://steamcommunity.com/openid/id/<17-digit SteamID64>`.
- Generate state with a cryptographically secure random source, store only server-side flow data, apply a short TTL, and consume state atomically once.
- Bind state must include the authenticated user identity and flow type; callback query values cannot select the target user or switch modes.
- Enforce uniqueness of `steam_id` in the database and translate uniqueness conflicts to `steam_account_conflict` without leaking another user's identity.
- Validate `authorization_url` server-side as a Steam HTTPS OpenID endpoint before returning it. The frontend uses it only for top-level navigation and never handles it as an authentication token.
- Reuse normal access-token issuance, refresh-token hashing/rotation, cookie attributes, revocation, and `auth_version` checks.
- Set the refresh cookie before redirecting to the frontend. Use `HttpOnly`, `Secure` in production, the narrowest practical `Path`, and the existing `SameSite` policy.
- Apply existing rate limits to login/bind starts and callback verification. Do not log Steam API keys, OpenID assertions, one-time state values, session tokens, or unbind passwords.
- Treat Steam persona, avatar, profile URL, and country as untrusted upstream data: validate lengths and formats and render them as text or validated URLs.
- A Steam-only account must gain another verified login method before Steam can be unbound.

## Session policy

| Token   | TTL                                                            | Storage                            |
| ------- | -------------------------------------------------------------- | ---------------------------------- |
| Access  | `ACCESS_TOKEN_TTL_SECONDS` (default 900 seconds)               | Browser memory only                |
| Refresh | `REFRESH_TOKEN_TTL_SECONDS` (default 864000 seconds / 10 days) | HttpOnly cookie + server-side hash |

Steam login only seeds the normal refresh-cookie session. Startup and completion both recover an access token through `/auth/refresh`; no separate Steam token store exists in the browser.

## Configuration

| Environment variable         | Purpose                                                                |
| ---------------------------- | ---------------------------------------------------------------------- |
| `STEAM_API_KEY`              | Server-only Steam Web API key for player summaries                     |
| `STEAM_OPENID_REALM`         | Exact OpenID realm, normally the public site origin                    |
| `STEAM_RETURN_URL`           | Absolute API callback URL                                              |
| `STEAM_WEB_ORIGIN`           | Allowed frontend origin for completion redirects                       |
| `STEAM_WEB_API_KEY`          | Alias for `STEAM_API_KEY`; values must match if both are set           |
| `STEAM_PROXY_URL`            | Optional outbound HTTP(S) proxy used only for Steam requests           |
| `STEAM_HTTP_TIMEOUT_SECONDS` | Steam request timeout in seconds; defaults to 15 (allowed range 1–120) |
| `COOKIE_DOMAIN`              | Optional refresh-cookie domain; empty keeps it host-only               |

These values are server configuration. None should use a `NEXT_PUBLIC_` prefix or be added to the web application's environment. For the current production topology use `STEAM_OPENID_REALM=https://chatapi.cngokz.com`, `STEAM_RETURN_URL=https://chatapi.cngokz.com/auth/steam/callback`, and `STEAM_WEB_ORIGIN=https://chat.cngokz.com`. If the production host cannot connect to Steam directly, configure `STEAM_PROXY_URL` with an outbound HTTP(S) proxy reachable by the API process; do not point it at the site's reverse proxy.

## Out of scope

- Linking multiple Steam accounts to one forum account
- Steam inventory, trade, ownership, or entitlement features
- Mobile Steam app deep links
