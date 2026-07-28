#!/bin/sh
set -eu

# Restore a gzip SQL dump into the compose postgres service.
# Usage:
#   ./scripts/backup/restore-postgres.sh /backups/lumiforum-lumiforum-YYYYMMDD.sql.gz
#
# WARNING: destructive. Stops API writes first.

FILE="${1:?usage: restore-postgres.sh <dump.sql.gz>}"
COMPOSE="${COMPOSE:-docker compose -f docker-compose.prod.yml --env-file .env}"

if [ ! -f "${FILE}" ]; then
  echo "dump not found: ${FILE}" >&2
  exit 1
fi

: "${POSTGRES_USER:?set POSTGRES_USER in env}"
: "${POSTGRES_DB:=lumiforum}"

echo "[restore] stopping api to pause writes"
${COMPOSE} stop api || true

echo "[restore] restoring ${FILE}"
gunzip -c "${FILE}" | ${COMPOSE} exec -T postgres \
  psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -v ON_ERROR_STOP=1

echo "[restore] starting api"
${COMPOSE} start api
echo "[restore] done — verify /ready and application smoke tests"
