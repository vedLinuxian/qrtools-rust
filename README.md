<div align="center">

# ▦ QRTools

### Advanced QR Code Platform — Powered by Rust + Axum

[![CI](https://github.com/vedLinuxian/qrtools-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/vedLinuxian/qrtools-rust/actions)
[![Release](https://img.shields.io/github/v/release/vedLinuxian/qrtools-rust?color=58a6ff&label=release)](https://github.com/vedLinuxian/qrtools-rust/releases)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg?logo=rust)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ed?logo=docker)](https://github.com/vedLinuxian/qrtools-rust/pkgs/container/qrtools-rust)

[Live Demo](https://vedlinuxian.github.io/qrtools-rust) · [API Docs](#api-reference) · [Docker](#docker) · [Changelog](CHANGELOG.md)

</div>

---

QRTools is an **open-source, self-hostable QR code platform** built entirely in Rust. Unlike single-purpose QR generators, it provides a full-featured REST API and an advanced single-page dashboard with features that no other open-source tool combines in one binary.

---

## ✨ Features

### QR Engine
| Feature | Details |
|---|---|
| **Module styles** | Square · Dot · Rounded — per-request choice |
| **Logo overlay** | Embed any PNG/JPEG logo at the center of the QR code |
| **SVG gradients** | Linear gradient between two colors on SVG output |
| **Output formats** | PNG · JPEG (configurable quality) · WebP · SVG |
| **Badge label** | Render text below the QR code using embedded DejaVu Sans font |
| **Error correction** | L (7%) / M (15%) / Q (25%) / H (30%) |

### Payload Builder
Generates correctly-formatted QR strings for **10 payload types** — all properly escaped:

| Type | Format |
|---|---|
| **WiFi** | `WIFI:T:WPA;S:…;P:…;H:false;;` — WPA/WEP/open |
| **vCard 3.0** | Full RFC 2426 contact with name, org, phone, email, url, address, note |
| **SMS** | `sms:+1234?body=…` |
| **Email** | `mailto:…?subject=…&body=…&cc=…` |
| **Geolocation** | `geo:lat,lon[,alt]` |
| **Phone** | `tel:+1234` |
| **URL / Text** | Raw passthrough |
| **Calendar Event** | RFC 5545 VCALENDAR/VEVENT with DTSTART, DTEND, SUMMARY, LOCATION |
| **Bitcoin** | `bitcoin:address?amount=…&label=…&message=…` |

### API & Infrastructure
| Feature | Details |
|---|---|
| **Batch encode** | Up to 200 items in one request, concurrent via Tokio |
| **ZIP export** | Download all batch QR codes as a single `.zip` archive |
| **URL decode** | Server-side fetches a remote image URL and decodes it |
| **Camera decode** | WebRTC + jsQR — scan directly from device camera in the browser |
| **Prometheus metrics** | `GET /metrics` — Prometheus text format, tracks encode/decode/batch/export |
| **WebSocket live stats** | `ws://host/ws/stats` — real-time ops counter pushed on every operation |
| **SQLite history** | All operations persisted; paginated `GET /api/history` with thumbnails |
| **Per-IP rate limiting** | Token-bucket via `governor` crate, configurable RPM |
| **API key auth** | Optional `X-API-Key` header enforcement |
| **Graceful shutdown** | SIGTERM + Ctrl-C drains in-flight requests before exit |
| **Dark/light theme** | System-preference-aware, persisted in `localStorage` |
| **Keyboard shortcuts** | `E/D/B/P/H` switch tabs; `Ctrl+Enter` submits; `T` toggles theme |

---

## 🚀 Quick Start

### From source

```bash
# Prerequisites: Rust 1.82+, SQLite dev headers (libsqlite3-dev on Debian/Ubuntu)
git clone https://github.com/vedLinuxian/qrtools-rust.git
cd qrtools-rust

# Optional: copy and edit config
cp .env.example .env

cargo run --release
# → http://localhost:8080
```

### Docker

```bash
docker compose up -d
# → http://localhost:8080
```

### Pre-built binary

Download the latest release binary from the [Releases page](https://github.com/vedLinuxian/qrtools-rust/releases) for your platform (Linux x86_64/aarch64, macOS, Windows):

```bash
chmod +x qrtools-web
./qrtools-web
```

---

## 🐳 Docker

```yaml
# docker-compose.yml already included
services:
  qrtools:
    image: ghcr.io/vedlinuxian/qrtools-rust:latest
    ports: ["8080:8080"]
    environment:
      RATE_LIMIT_RPM: "120"
      METRICS_ENABLED: "true"
    volumes:
      - ./data:/app/data
```

Multi-stage build produces a `< 50 MB` distroless image.

---

## ⚙️ Configuration

All configuration via environment variables. Create `.env` in the project root:

```env
# Server
HOST=0.0.0.0
PORT=8080
LOG_LEVEL=info
JSON_LOGGING=false

# Limits
MAX_UPLOAD_BYTES=10485760       # 10 MB
HISTORY_CAPACITY=500            # LRU cache entries

# Security
API_KEY=                        # leave blank to disable auth
RATE_LIMIT_RPM=120              # requests per minute per IP (0 = disabled)
CORS_ORIGINS=*                  # comma-separated origins

# Storage
DB_PATH=qrtools.db              # SQLite file path

# Observability
METRICS_ENABLED=true            # expose /metrics endpoint
MAX_REMOTE_FETCH_BYTES=5242880  # max bytes fetched for URL-based decode (5 MB)
```

---

## 📡 API Reference

All endpoints are under `/api/`. The server also serves the SPA from `/`.

### Encode

#### `POST /api/encode`

```jsonc
{
  "data": "https://example.com",        // required
  "format": "png",                       // "png" | "jpeg" | "webp" | "svg"
  "ec_level": "M",                       // "L" | "M" | "Q" | "H"
  "module_size": 10,                     // 1–50 px per module
  "quiet_zone": 4,                       // border modules
  "foreground": "#000000",
  "background": "#ffffff",
  "module_style": "square",             // "square" | "dot" | "rounded"
  "gradient_color": "#2563eb",          // SVG format only; second color
  "logo_base64": "<base64>",            // PNG/JPEG logo, EC=H recommended
  "logo_ratio": 0.22,                   // logo size as fraction of QR width
  "badge_label": "Scan Me!",            // text rendered below QR (PNG only)
  "jpeg_quality": 85                     // 10–100, JPEG only
}
```

Response:
```jsonc
{
  "id": "uuid",
  "mime_type": "image/png",
  "data": "<base64>",           // base64 image OR SVG string (also base64)
  "width": 232,
  "height": 232,
  "thumbnail": "<base64>",      // 48×48 PNG thumbnail
  "timestamp": "2026-03-13T…"
}
```

---

### Decode

#### `POST /api/decode/json`
```json
{ "image_b64": "<base64-encoded image>" }
```

#### `POST /api/decode/upload`
Multipart form upload. Field name: `file` or `image`.

#### `POST /api/decode/url`
```json
{ "url": "https://example.com/qr.png" }
```
Server fetches the image (respecting `MAX_REMOTE_FETCH_BYTES`) and decodes it.

Response (all three):
```json
{ "id": "uuid", "data": "decoded text", "format": "QR_CODE", "timestamp": "…" }
```

---

### Batch

#### `POST /api/batch/encode`
Returns JSON with per-item results.

```jsonc
{
  "defaults": {
    "format": "png",
    "ec_level": "M",
    "module_style": "dot"
  },
  "items": [
    { "data": "https://example.com" },
    { "data": "Hello World", "badge_label": "HW" }
  ]
}
```

Response: `{ "batch_id", "total", "success", "failed", "items": [...], "timestamp" }`
Each item contains: `{ "index", "data", "ok", "id", "mime_type", "image" (base64), "thumbnail", "error" }`

#### `POST /api/batch/export`
Same request body as above. Returns `application/zip` binary with one file per QR code.

---

### Payload Builder

#### `POST /api/payload/build`
Returns the formatted payload string without generating a QR image.

```json
{ "type": "wifi", "ssid": "HomeNet", "password": "secret", "auth": "WPA", "hidden": false }
```
Response: `{ "payload": "WIFI:T:WPA;S:HomeNet;P:secret;H:false;;" }`

#### `POST /api/payload/encode`
Builds the payload and immediately generates a QR code.

```jsonc
{
  "payload": { "type": "geo", "latitude": 48.8566, "longitude": 2.3522 },
  "options": { "format": "png", "ec_level": "H", "module_size": 10, "quiet_zone": 4,
               "foreground": "#000000", "background": "#ffffff" }
}
```
Response: same as `POST /api/encode` with additional `"payload"` field.

Complete list of payload types: `wifi` · `vcard` · `sms` · `email` · `geo` · `phone` · `url` · `text` · `cal_event` · `bitcoin`

---

### History

```
GET    /api/history?limit=50&offset=0    # paginated list
GET    /api/history/:id                  # single entry by UUID
DELETE /api/history/:id                  # delete one entry
DELETE /api/history                      # clear all (via history endpoint)
```

---

### Health & Observability

```
GET /api/health         # liveness  →  { status, version, timestamp }
GET /api/health/ready   # readiness →  { status, ops_total, history_entries }
GET /metrics            # Prometheus text format
WS  /ws/stats           # WebSocket: push ServerStats JSON on every operation
```

Prometheus counters: `qrtools_encode_total`, `qrtools_decode_total`, `qrtools_batch_total`, `qrtools_export_total`

---

## 🏗️ Architecture

```
qrtools-rust/
├── src/
│   ├── main.rs                    # Axum server, routing, SQLite init, shutdown
│   ├── lib.rs                     # Library target (for unit tests)
│   ├── config.rs                  # Environment-based configuration
│   ├── error.rs                   # Unified AppError → HTTP response
│   ├── state.rs                   # AppState: SQLitePool, moka cache, broadcast
│   ├── services/
│   │   ├── qr_engine.rs           # Core QR rendering (formats, styles, logo, badge)
│   │   └── payload_builder.rs     # 10 structured payload builders
│   ├── api/
│   │   ├── encode.rs              # POST /api/encode
│   │   ├── decode.rs              # POST /api/decode/{json,upload,url}
│   │   ├── batch.rs               # POST /api/batch/{encode,export}
│   │   ├── payload.rs             # POST /api/payload/{build,encode}
│   │   ├── history.rs             # GET/DELETE /api/history
│   │   ├── health.rs              # GET /api/health{,/ready}
│   │   ├── ws.rs                  # WS  /ws/stats
│   │   ├── metrics_handler.rs     # GET /metrics
│   │   └── mod.rs
│   └── middleware/
│       ├── rate_limit.rs          # Per-IP token bucket (governor)
│       ├── auth.rs                # X-API-Key enforcement
│       └── mod.rs
├── static/
│   ├── index.html                 # 6-tab SPA
│   ├── css/style.css              # Design system (dark/light)
│   └── js/app.js                  # Vanilla ES2022 + jsQR (CDN)
├── assets/
│   └── DejaVuSans.ttf             # Embedded font for badge labels
├── docs/
│   └── index.html                 # GitHub Pages landing page
├── tests/
│   └── integration_test.rs
├── Dockerfile
├── docker-compose.yml
└── .github/workflows/
    ├── ci.yml
    └── release.yml
```

### Technology Stack

| Layer | Crate / Tool |
|---|---|
| Web framework | `axum 0.8` |
| Async runtime | `tokio 1` |
| QR encoding | `qrcode 0.14` |
| QR decoding | `rxing 0.6` |
| Image processing | `image 0.25`, `imageproc 0.25` |
| Font rendering | `ab_glyph 0.2` |
| Database | `sqlx 0.8` + SQLite |
| In-memory cache | `moka 0.12` |
| Rate limiting | `governor 0.6` |
| HTTP client | `reqwest 0.12` |
| ZIP archive | `zip 2` |
| Metrics | `metrics` + `metrics-exporter-prometheus 0.15` |
| Serialisation | `serde` + `serde_json` |
| Tracing | `tracing` + `tracing-subscriber` |

---

## 🧪 Development

```bash
# Dev run with auto-reload
cargo install cargo-watch
cargo watch -x run

# Tests
cargo test

# Lint + format
cargo clippy --all-targets -- -D warnings
cargo fmt

# Audit dependencies
cargo install cargo-audit
cargo audit
```

---

## 🔒 Security

- **Rate limiting** — per-IP token bucket, configurable via `RATE_LIMIT_RPM`
- **API key** — optional `X-API-Key` header on all `/api/` routes
- **Body size cap** — `RequestBodyLimitLayer` prevents oversized uploads
- **Remote fetch cap** — `MAX_REMOTE_FETCH_BYTES` limits URL-based decode payloads
- **Distroless Docker** — runtime image has no shell, no package manager
- **`cargo audit`** in CI — blocks merges on known advisories

---

## 🤝 Contributing

Issues and PRs are welcome. Please run `cargo fmt` and `cargo clippy` before submitting.

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

---

<div align="center">

**Star ⭐ the repo if you find it useful!**

Made with ❤️ and Rust

</div>
