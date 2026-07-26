# Web authentication architecture

## Token storage

- Access tokens exist only in JavaScript memory.
- Refresh tokens exist only in the API's host-only HttpOnly cookie.
- No authentication value is written to LocalStorage or SessionStorage.

## Bootstrap

`AuthProvider` starts in `loading`, calls `/auth/refresh`, then `/auth/me`.
The UI renders route-guard loading states until the result is known, preventing
protected content from flashing before authentication is resolved.

## Refresh behavior

- Access tokens refresh shortly before expiry.
- Protected requests that receive `401` refresh and retry exactly once.
- Concurrent refreshes in one tab share one Promise.
- Web Locks serialize refresh rotation across tabs when the browser provides
  the API (production HTTPS contexts).
- A failed refresh clears in-memory state and moves the UI to unauthenticated.

## Route policy

- `/profile` is wrapped by `RequireAuth`.
- `/login` and `/register` are wrapped by `GuestOnly`.
- Guards are client-side because the refresh cookie belongs to the separate API
  origin and the web server never receives the in-memory access token.
- Backend middleware remains the security boundary; client guards provide only
  navigation and rendering behavior.
