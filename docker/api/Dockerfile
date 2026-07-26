# Production multi-stage build for apps/api
FROM rust:1.88-bookworm AS builder

WORKDIR /workspace

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
  && rm -rf /var/lib/apt/lists/*

# Dependency layer cache
COPY Cargo.toml Cargo.lock* ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
RUN mkdir -p apps/api/src \
  && echo "fn main() {}" > apps/api/src/main.rs \
  && cargo build --release -p lumiforum-api \
  && rm -rf apps/api/src

COPY apps/api apps/api
RUN touch apps/api/src/main.rs \
  && cargo build --release -p lumiforum-api

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 10001 appuser

WORKDIR /app

COPY --from=builder /workspace/target/release/lumiforum-api /usr/local/bin/lumiforum-api

USER appuser

ENV APP_ENV=production \
    HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/health || exit 1

CMD ["lumiforum-api"]
