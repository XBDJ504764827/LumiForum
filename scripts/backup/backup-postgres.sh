#!/bin/sh
set -eu

: "${POSTGRES_USER:?POSTGRES_USER is required}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}"
: "${POSTGRES_DB:=lumiforum}"
: "${BACKUP_RETENTION_DAYS:=14}"
: "${BACKUP_DIR:=/backups}"

export PGPASSWORD="${POSTGRES_PASSWORD}"
HOST="${POSTGRES_HOST:-postgres}"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
FILE="${BACKUP_DIR}/lumiforum-${POSTGRES_DB}-${STAMP}.sql.gz"
TMP="${FILE}.tmp"

mkdir -p "${BACKUP_DIR}"

echo "[backup] starting ${FILE}"
pg_dump \
  --host="${HOST}" \
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
