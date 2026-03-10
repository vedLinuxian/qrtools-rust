# QR Tools Web Dashboard

[![CI](https://github.com/vedLinuxian/qrtools-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/vedLinuxian/qrtools-rust/actions)

A **production-grade** QR code encode / decode web dashboard built with **Rust + Axum**. Features a modern dark/light single-page dashboard served directly by the Rust binary — zero Node.js required.

![Dashboard Screenshot](assets/screenshot.png)

---

## Features

| Feature | Details |
|--------|---------|
| **Encode** | Text / URL → PNG or SVG QR codes |
| **Decode** | Upload or paste base64 image → decoded text |
| **Batch** | Encode up to 50 items at once, download all |
| **History** | In-memory LRU cache of recent operations |
| **Colors** | Custom foreground & background via CSS color strings |
| **Error Correction** | L / M / Q / H levels |
| **QR Variants** | Normal QR codes (Micro QR / rMQR via CLI extensions) |
| **API** | Clean REST JSON API at `/api/` |
| **Health** | `/api/health` (liveness) & `/api/health/ready` (readiness) |
| **CORS** | Configurable origins |
| **Compression** | Gzip / Brotli response compression |
| **Rate Limiting** | Configurable request body size limit |
| **Docker** | Multi-stage image `< 50 MB` |
| **CI/CD** | GitHub Actions — lint, test, audit, Docker, multi-arch release |

---

## Quick Start

### From source

```bash
git clone https://github.com/vedLinuxian/qrtools-rust.git
cd qrtools-rust
cp .env.example .env
cargo run --release
# Open http://localhost:8080
```

### Docker

```bash
docker compose up -d
# Open http://localhost:8080
```

---

## API Reference

### `POST /api/encode`

```json
{
  "data": "https://example.com",
  "format": "png",          // "png" | "svg"
  "ec_level": "M",          // "L" | "M" | "Q" | "H"
  "module_size": 10,        // pixels per module (1–50)
  "quiet_zone": 4,          // border modules
  "foreground": "#000000",  // CSS color
  "background": "#ffffff"   // CSS color
}
```

Response: `{ "id", "mime_type", "data" (base64/SVG), "width", "height", "timestamp" }`

---

### `POST /api/decode/json`

```json
{
  "image_b64": "<base64-encoded image>"
}
```

Response: `{ "id", "text", "format", "timestamp" }`

---

### `POST /api/decode/upload`

Multipart form with a field named `file` or `image`.

---

### `GET /api/history`

Returns the 100 most recent operations from the in-memory LRU cache.

---

### `DELETE /api/history`

Clears the operation history cache.

---

### `GET /api/health`

Liveness probe — returns `{ "status": "ok", "version", "timestamp" }`.

### `GET /api/health/ready`

Readiness probe — includes `ops_total` and `history_entries` counts.

---

## Configuration

All configuration is via environment variables (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | Port |
| `LOG_LEVEL` | `info` | `error`/`warn`/`info`/`debug`/`trace` |
| `JSON_LOGGING` | `false` | Structured JSON logs |
| `MAX_UPLOAD_BYTES` | `10485760` | Max request body (10 MB) |
| `HISTORY_CAPACITY` | `500` | LRU cache size |
| `CORS_ORIGINS` | `*` | Comma-separated allowed origins |

---

## Development

```bash
# Install dev tools
rustup component add clippy rustfmt

# Run with hot reload (cargo-watch)
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Lint
cargo clippy --all-targets -- -D warnings

# Format
cargo fmt
```

---

## Architecture

```
qrtools-rust/
├── src/
│   ├── main.rs          # Axum server, routing, middleware
│   ├── config.rs        # Environment-based configuration
│   ├── error.rs         # Unified error type with HTTP mapping
│   ├── state.rs         # Shared application state (cache, metrics)
│   └── api/
│       ├── encode.rs    # POST /api/encode
│       ├── decode.rs    # POST /api/decode/{json,upload}
│       ├── health.rs    # GET  /api/health{,/ready}
│       └── history.rs   # GET/DELETE /api/history
├── static/
│   ├── index.html       # Single-page dashboard
│   ├── css/style.css    # Dark/light theme CSS
│   └── js/app.js        # Vanilla ES2022 frontend
├── tests/
│   └── integration_test.rs
├── Dockerfile
├── docker-compose.yml
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

---

## License

MIT OR Apache-2.0

---

> Inspired by [qrtool](https://github.com/sorairolake/qrtool) — extended with a full web dashboard.
