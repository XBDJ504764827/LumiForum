#!/usr/bin/env bash
set -euo pipefail

# Post-deploy smoke checks against the public origin or local edge.
BASE_URL="${1:-http://127.0.0.1}"

echo "[smoke] base=${BASE_URL}"
curl -fsS "${BASE_URL}/health" | head -c 200
echo
curl -fsS "${BASE_URL}/ready" | head -c 200
echo
curl -fsS -o /dev/null -w "home=%{http_code}\n" "${BASE_URL}/"
curl -fsS -o /dev/null -w "robots=%{http_code}\n" "${BASE_URL}/robots.txt"
curl -fsS -o /dev/null -w "sitemap=%{http_code}\n" "${BASE_URL}/sitemap.xml"
curl -fsS -o /dev/null -w "api_topics=%{http_code}\n" "${BASE_URL}/api/topics?page=1&page_size=1"
echo "[smoke] ok"
