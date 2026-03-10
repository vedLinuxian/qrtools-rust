# ── Stage 1: Build ──────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

# Install C toolchain and OpenSSL dev headers
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libclang-dev \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Layer dependency cache: copy manifests first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

# Copy real sources
COPY src ./src
# Touch main.rs so Cargo rebuilds it
RUN touch src/main.rs && cargo build --release --locked

# ── Stage 2: Runtime ────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/qrtools-web /usr/local/bin/qrtools-web

# Copy static assets
COPY static ./static

# Expose port
EXPOSE 8080

# Unprivileged user
RUN useradd --system --no-create-home --shell /usr/sbin/nologin appuser
USER appuser

ENV HOST=0.0.0.0
ENV PORT=8080
ENV LOG_LEVEL=info
ENV JSON_LOGGING=true

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -qO- http://localhost:8080/api/health || exit 1

ENTRYPOINT ["/usr/local/bin/qrtools-web"]
