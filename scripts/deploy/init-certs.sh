#!/usr/bin/env bash
set -euo pipefail

# Issue the first Let's Encrypt certificate, then reload nginx with TLS config.
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "${ROOT}"

ENV_FILE="${ENV_FILE:-.env}"
COMPOSE="docker compose -f docker-compose.prod.yml --env-file ${ENV_FILE}"

# shellcheck disable=SC1090
set -a
source "${ENV_FILE}"
set +a

: "${DOMAIN:?DOMAIN is required}"
: "${CERTBOT_EMAIL:?CERTBOT_EMAIL is required}"

echo "[certs] ensuring nginx is up for ACME HTTP-01"
${COMPOSE} up -d nginx

echo "[certs] requesting certificate for ${DOMAIN}"
${COMPOSE} run --rm --profile certs certbot certonly \
  --webroot -w /var/www/certbot \
  -d "${DOMAIN}" \
  --email "${CERTBOT_EMAIL}" \
  --agree-tos \
  --non-interactive \
  --rsa-key-size 4096

echo "[certs] recreating nginx with TLS template"
${COMPOSE} up -d --force-recreate nginx

echo "[certs] done"
echo "Install renew cron on host, e.g.:"
echo "  0 4 * * * cd ${ROOT} && docker compose -f docker-compose.prod.yml --env-file ${ENV_FILE} run --rm --profile certs certbot renew && docker compose -f docker-compose.prod.yml --env-file ${ENV_FILE} exec nginx nginx -s reload"
