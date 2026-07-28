#!/bin/sh
set -eu

# Lightweight cron loop without installing cron packages.
# Runs backup immediately once, then sleeps until next schedule approximation.
: "${BACKUP_CRON:=0 3 * * *}"
: "${BACKUP_RETENTION_DAYS:=14}"

echo "[backup] entrypoint ready cron='${BACKUP_CRON}' retention=${BACKUP_RETENTION_DAYS}d"

# Parse "m h * * *" only (minute hour). Full cron grammar is not required for phase 12.
MINUTE="$(echo "${BACKUP_CRON}" | awk '{print $1}')"
HOUR="$(echo "${BACKUP_CRON}" | awk '{print $2}')"

if [ "${MINUTE}" = "*" ] || [ "${HOUR}" = "*" ]; then
  echo "[backup] unsupported cron expression; defaulting to daily 03:00 UTC"
  MINUTE=0
  HOUR=3
fi

/usr/local/bin/backup-postgres.sh || echo "[backup] initial run failed"

while true; do
  NOW_H="$(date -u +%H)"
  NOW_M="$(date -u +%M)"
  # Sleep until target hour:minute UTC
  TARGET=$((10#${HOUR} * 3600 + 10#${MINUTE} * 60))
  NOW=$((10#${NOW_H} * 3600 + 10#${NOW_M} * 60))
  SLEEP=$((TARGET - NOW))
  if [ "${SLEEP}" -le 0 ]; then
    SLEEP=$((SLEEP + 86400))
  fi
  echo "[backup] next run in ${SLEEP}s"
  sleep "${SLEEP}"
  /usr/local/bin/backup-postgres.sh || echo "[backup] scheduled run failed"
  # Avoid double-fire within the same minute
  sleep 60
done
