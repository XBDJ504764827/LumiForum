#!/bin/sh
set -eu

# Restore a gzip SQL dump into the host PostgreSQL instance.
# Usage (on the production host):
#   BACKUP_ENV=/opt/lumiforum/env/backup.env \
#     ./scripts/backup/restore-postgres.sh /opt/lumiforum/backups/lumiforum-lumiforum-YYYYMMDD.sql.gz
#
# WARNING: destructive. Stops the API first.

FILE="${1:?usage: restore-postgres.sh <dump.sql.gz>}"

if [ ! -f "${FILE}" ]; then
  echo "dump not found: ${FILE}" >&2
  exit 1
fi

if [ -n "${BACKUP_ENV:-}" ] && [ -f "${BACKUP_ENV}" ]; then
  # shellcheck disable=SC1091
  . "${BACKUP_ENV}"
fi
if [ -f /etc/lumiforum/backup.env ]; then
  # shellcheck disable=SC1091
  . /etc/lumiforum/backup.env
fi

: "${POSTGRES_USER:?set POSTGRES_USER in env}"
: "${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD in env}"
: "${POSTGRES_DB:=lumiforum}"
: "${POSTGRES_HOST:=127.0.0.1}"

echo "[restore] stopping api to pause writes"
systemctl --user stop lumiforum-api 2>/dev/null || systemctl stop lumiforum-api 2>/dev/null || true

echo "[restore] restoring ${FILE}"
gunzip -c "${FILE}" | PGPASSWORD="${POSTGRES_PASSWORD}" psql \
  --host="${POSTGRES_HOST}" \
  --username="${POSTGRES_USER}" \
  --dbname="${POSTGRES_DB}" \
  -v ON_ERROR_STOP=1

echo "[restore] starting api"
systemctl --user start lumiforum-api 2>/dev/null || systemctl start lumiforum-api 2>/dev/null || true
echo "[restore] done — verify /ready and application smoke tests"
