# Production multi-stage build for apps/api
FROM rust:1.88-bookworm AS builder

WORKDIR /workspace

RUN apt-get update \
  && apt-get install -y --no-install-recommends pkg-config libssl-dev build-essential ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# Dependency layer cache
COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src \
  && printf 'fn main() {}\n' > apps/api/src/main.rs \
  && printf '// placeholder\n' > apps/api/src/lib.rs \
  && cargo build --release -p lumiforum-api \
  && rm -rf apps/api/src

COPY apps/api apps/api
RUN touch apps/api/src/main.rs apps/api/src/lib.rs \
  && cargo build --release -p lumiforum-api \
  && strip target/release/lumiforum-api \
  && strip target/release/migrate

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates curl libssl3 tini \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 10001 appuser \
  && mkdir -p /data/uploads \
  && chown -R appuser:appuser /data

WORKDIR /app

COPY --from=builder /workspace/target/release/lumiforum-api /usr/local/bin/lumiforum-api
COPY --from=builder /workspace/target/release/migrate /usr/local/bin/lumiforum-migrate

USER appuser

ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=info,tower_http=info,sqlx=warn \
    STORAGE_LOCAL_ROOT=/data/uploads

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=25s --retries=5 \
  CMD curl -fsS http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["lumiforum-api"]
