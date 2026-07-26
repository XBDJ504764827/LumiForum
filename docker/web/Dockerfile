# Production multi-stage build for apps/web
FROM node:20-bookworm-slim AS deps

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

WORKDIR /workspace

COPY package.json pnpm-workspace.yaml pnpm-lock.yaml* .npmrc ./
COPY apps/web/package.json apps/web/package.json
COPY packages/ui/package.json packages/ui/package.json
COPY packages/types/package.json packages/types/package.json
COPY packages/shared/package.json packages/shared/package.json

RUN pnpm install --frozen-lockfile=false

FROM node:20-bookworm-slim AS builder

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate

WORKDIR /workspace

COPY --from=deps /workspace/node_modules ./node_modules
COPY --from=deps /workspace/apps/web/node_modules ./apps/web/node_modules
COPY --from=deps /workspace/packages ./packages
COPY . .

ENV NEXT_TELEMETRY_DISABLED=1 \
    NODE_ENV=production

RUN pnpm --filter @lumiforum/web build

FROM node:20-bookworm-slim AS runtime

RUN corepack enable && corepack prepare pnpm@9.15.0 --activate \
  && apt-get update \
  && apt-get install -y --no-install-recommends wget ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 10001 appuser

WORKDIR /app

ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0

COPY --from=builder /workspace/apps/web/.next/standalone ./
COPY --from=builder /workspace/apps/web/.next/static ./apps/web/.next/static
COPY --from=builder /workspace/apps/web/public ./apps/web/public

USER appuser

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD wget -qO- http://127.0.0.1:3000 >/dev/null 2>&1 || exit 1

CMD ["node", "apps/web/server.js"]
