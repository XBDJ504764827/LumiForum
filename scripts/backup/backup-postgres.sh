#!/bin/sh
set -eu

# Host-side Postgres backup (run by cron, as the deploying user).
# Reads credentials from the env file given by BACKUP_ENV (or the legacy
# /etc/lumiforum/backup.env); overridable via environment variables.

if [ -n "${BACKUP_ENV:-}" ] && [ -f "${BACKUP_ENV}" ]; then
  # shellcheck disable=SC1091
  . "${BACKUP_ENV}"
fi
if [ -f /etc/lumiforum/backup.env ]; then
  # shellcheck disable=SC1091
  . /etc/lumiforum/backup.env
fi

: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"
: "${POSTGRES_DB:=lumiforum}"
: "${POSTGRES_HOST:=127.0.0.1}"
: "${BACKUP_RETENTION_DAYS:=14}"
: "${BACKUP_DIR:=${HOME}/lumiforum/backups}"

export PGPASSWORD="${POSTGRES_PASSWORD}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
FILE="${BACKUP_DIR}/lumiforum-${POSTGRES_DB}-${STAMP}.sql.gz"
TMP="${FILE}.tmp"

mkdir -p "${BACKUP_DIR}"

echo "[backup] starting ${FILE}"
pg_dump \
  --host="${POSTGRES_HOST}" \
  --username="${POSTGRES_USER}" \
  --dbname="${POSTGRES_DB}" \
  --format=plain \
  --no-owner \
  --no-acl \
  | gzip -c > "${TMP}"

mv "${TMP}" "${FILE}"
echo "[backup] wrote ${FILE} ($(wc -c < "${FILE}") bytes)"

# Retention
find "${BACKUP_DIR}" -type f -name "lumiforum-*.sql.gz" -mtime "+${BACKUP_RETENTION_DAYS}" -print -delete || true
echo "[backup] retention applied (${BACKUP_RETENTION_DAYS} days)"
