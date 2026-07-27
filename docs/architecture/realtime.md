# Realtime Architecture

**Status:** Accepted for phase 10  
**Scope:** WebSocket gateway, multi-instance fan-out, presence, live notifications

## Goals

- Deliver inbox notifications to connected clients without polling.
- Support multiple API instances behind a load balancer.
- Keep durable notification storage in PostgreSQL; WebSocket is a delivery channel only.
- Bound connection growth with auth, heartbeats, and per-user connection limits.

## Topology

```text
Browser  --WS /ws-->  API instance A (local Hub)
                         | publish
                       Redis channel realtime:user-events
                         | subscribe
                       API instance B (local Hub) --> other sockets
```

Each process owns only its local sockets. Cross-instance delivery always goes through Redis Pub/Sub so any instance can push to a user connected elsewhere.

## Authentication

- Endpoint: `GET /ws?access_token=<jwt>`
- Token is validated with the existing access-token JWT pipeline and authorization snapshot.
- Unauthenticated or inactive users are rejected during the upgrade handshake.
- Connection rate limits use Redis counters keyed by peer IP.

## Client protocol

All frames are JSON text messages:

```json
{
  "type": "notification.created",
  "timestamp": "2026-07-27T12:00:00Z",
  "data": {}
}
```

Server → client event types:

| type                   | purpose                             |
| ---------------------- | ----------------------------------- |
| `connected`            | handshake ack with user id          |
| `pong`                 | heartbeat response                  |
| `notification.created` | new inbox notification payload      |
| `presence.updated`     | optional presence change for a user |
| `error`                | recoverable protocol error          |

Client → server:

| type                 | purpose                                    |
| -------------------- | ------------------------------------------ |
| `ping`               | heartbeat                                  |
| `subscribe.presence` | watch presence for user ids (bounded list) |

## Presence

Redis keys:

- `presence:online:{user_id}` → `"1"` with TTL (default 60s)
- refreshed on connect and every successful heartbeat

A user is online while the key exists. Multi-tab / multi-instance is handled by TTL refresh rather than reference counting, which avoids cross-instance decrements.

API:

- `GET /users/{id}/presence` → `{ online, last_seen_at }`

`last_seen_at` is the Redis key TTL-derived stamp when available, otherwise null.

## Notification integration

1. Domain services emit `NotificationEvent` as today.
2. `NotificationService` writes PostgreSQL and invalidates unread cache.
3. After a successful insert, it publishes a realtime envelope to Redis.
4. Every API instance receives the envelope and delivers only to local sockets for that user.

If publish fails, the durable notification still exists; the client recovers on reconnect / next poll.

## Connection lifecycle

1. Upgrade + JWT auth
2. Enforce per-user connection limit (default 5)
3. Register socket in local Hub
4. Mark presence online
5. Heartbeat loop; idle timeout closes the socket
6. On close: unregister; presence expires via TTL if no other connections refresh it

## Security

- JWT required
- Per-IP connection open rate limit
- Per-user max concurrent sockets
- Presence subscribe lists capped
- No unauthenticated broadcast channels

## Out of scope

- Chat / rooms
- Binary frames
- Sticky sessions (not required because of Redis fan-out)
- Full offline message queue beyond existing notifications table
