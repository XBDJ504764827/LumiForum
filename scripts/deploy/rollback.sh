#!/usr/bin/env bash
set -euo pipefail

# Rollback API/Web images to a previous tag or digest.
# Usage:
#   IMAGE_TAG=sha-abc123 ./scripts/deploy/rollback.sh
#   IMAGE_API=ghcr.io/org/lumiforum-api:v1.2.3 IMAGE_WEB=ghcr.io/org/lumiforum-web:v1.2.3 ./scripts/deploy/rollback.sh

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ENV_FILE:-.env}"
COMPOSE="docker compose -f docker-compose.prod.yml --env-file ${ENV_FILE}"

if [[ -n "${IMAGE_TAG:-}" ]]; then
  REGISTRY="${IMAGE_REGISTRY:-ghcr.io/lumiforum}"
  export IMAGE_API="${REGISTRY}/lumiforum-api:${IMAGE_TAG}"
  export IMAGE_WEB="${REGISTRY}/lumiforum-web:${IMAGE_TAG}"
fi

: "${IMAGE_API:?set IMAGE_API or IMAGE_TAG}"
: "${IMAGE_WEB:?set IMAGE_WEB or IMAGE_TAG}"

echo "[rollback] api=${IMAGE_API}"
echo "[rollback] web=${IMAGE_WEB}"

${COMPOSE} pull api web
${COMPOSE} up -d --no-deps api web nginx

echo "[rollback] waiting for health"
for _ in $(seq 1 40); do
  if curl -fsS http://127.0.0.1/health >/dev/null 2>&1; then
    echo "[rollback] healthy"
    ${COMPOSE} ps
    exit 0
  fi
  sleep 3
done

echo "[rollback] health check failed" >&2
exit 1
