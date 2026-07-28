#!/usr/bin/env bash
set -euo pipefail

# Bring up / update the production stack on the host.
# Expects: repository checkout + .env (from .env.production.example)

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ENV_FILE:-.env}"
COMPOSE="docker compose -f docker-compose.prod.yml --env-file ${ENV_FILE}"

if [[ ! -f "${ENV_FILE}" ]]; then
  echo "missing ${ENV_FILE}; copy .env.production.example first" >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
source "${ENV_FILE}"
set +a

: "${DOMAIN:?DOMAIN is required}"
: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"
: "${JWT_SECRET:?JWT_SECRET is required}"
: "${CORS_ORIGIN:?CORS_ORIGIN is required}"

echo "[deploy] pulling images"
${COMPOSE} pull api web nginx postgres redis || true

echo "[deploy] starting data plane"
${COMPOSE} up -d postgres redis

echo "[deploy] waiting for postgres"
for _ in $(seq 1 60); do
  if ${COMPOSE} exec -T postgres pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB:-lumiforum}" >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

echo "[deploy] running migrations"
${COMPOSE} run --rm --no-deps \
  -e DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/${POSTGRES_DB:-lumiforum}" \
  --entrypoint /usr/local/bin/lumiforum-migrate \
  api

echo "[deploy] starting application + edge"
${COMPOSE} up -d api web nginx backup

echo "[deploy] health checks"
for _ in $(seq 1 60); do
  if ${COMPOSE} exec -T api curl -fsS http://127.0.0.1:8080/ready >/dev/null 2>&1 \
    && ${COMPOSE} exec -T web wget -qO- http://127.0.0.1:3000 >/dev/null 2>&1; then
    # Edge may be HTTP bootstrap or HTTPS; try both without failing the loop body hard
    if curl -fsS "http://127.0.0.1/health" >/dev/null 2>&1 \
      || curl -kfsS "https://127.0.0.1/health" >/dev/null 2>&1 \
      || curl -fsS "https://${DOMAIN}/health" >/dev/null 2>&1; then
      echo "[deploy] healthy"
      ${COMPOSE} ps
      exit 0
    fi
  fi
  sleep 3
done

echo "[deploy] health check timed out" >&2
${COMPOSE} ps
${COMPOSE} logs --tail=80 api web nginx || true
exit 1
