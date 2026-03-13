# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

---

## [2.0.0] — 2026-03-13

This is a full rewrite of the platform. The v1 CLI-based tool and simple web prototype have been replaced with a professional, production-ready REST API server and SPA dashboard built on **Axum 0.8** and **Tokio**.

### Added — QR Engine
- **Module styles**: square (default), dot, rounded — per-request via `module_style`
- **Logo overlay**: embed any PNG/JPEG logo centered over the QR code; `logo_base64` + `logo_ratio` fields; automatically raises EC to H when not specified
- **SVG linear gradient**: two-color gradient fill via `gradient_color` on SVG output format
- **Badge label**: render arbitrary text below the QR code using the embedded DejaVu Sans font (`ab_glyph 0.2`); `badge_label` field in encode request
- **JPEG and WebP** output formats in addition to PNG and SVG; `jpeg_quality` (10–100) control
- **Quiet-zone** configurability: `quiet_zone` field (modules of blank border)

### Added — Payload Builder (new subsystem)
Ten structured payload builders with proper escaping and formatting:
- `wifi` — WPA/WEP/open with SSID, password, hidden flag
- `vcard` — RFC 2426 vCard 3.0: name, org, title, phone, email, url, address, note
- `sms` — `sms:` URI with body
- `email` — `mailto:` URI with cc, subject, body
- `geo` — `geo:` URI with latitude, longitude, optional altitude
- `phone` — `tel:` URI
- `url` / `text` — raw passthrough
- `cal_event` — RFC 5545 VCALENDAR/VEVENT: DTSTART, DTEND, SUMMARY, LOCATION, DESCRIPTION
- `bitcoin` — BIP-21 `bitcoin:` URI with amount, label, message

`POST /api/payload/build` — return formatted string only  
`POST /api/payload/encode` — build payload and immediately generate QR image

### Added — Batch Processing
- **Concurrent batch encode** (`POST /api/batch/encode`): up to 200 items, Tokio-parallel, `defaults` + per-item override model
- **ZIP export** (`POST /api/batch/export`): same body as batch encode, returns `application/zip` binary with one image file per item
- Batch response: `{ batch_id, total, success, failed, items[], timestamp }`

### Added — Decode
- `POST /api/decode/json` — base64 image body
- `POST /api/decode/upload` — multipart file upload
- `POST /api/decode/url` — server-side URL fetch + decode (`MAX_REMOTE_FETCH_BYTES` cap via `reqwest`)
- All decode paths backed by `rxing 0.6`

### Added — Infrastructure & Observability
- **Prometheus metrics** at `GET /metrics` — counters: `qrtools_encode_total`, `qrtools_decode_total`, `qrtools_batch_total`, `qrtools_export_total`
- **WebSocket live stats** at `ws://host/ws/stats` — broadcasts `ServerStats` JSON on every operation via `tokio::sync::broadcast`
- **SQLite persistence** (`sqlx 0.8`, runtime queries) with `moka 0.12` LRU in-memory layer for history
- **Paginated history** `GET /api/history?limit=&offset=` with thumbnail thumbnails
- **Per-IP rate limiting** via `governor 0.6` token-bucket; configurable via `RATE_LIMIT_RPM` env var
- **Optional API key auth** via `X-API-Key` header; enabled by setting `API_KEY` env var
- **Liveness + readiness endpoints**: `GET /api/health` and `GET /api/health/ready`
- **Graceful shutdown** on SIGTERM and Ctrl-C: drains in-flight requests before exit
- **`RequestBodyLimitLayer`** capping upload body size to `MAX_UPLOAD_BYTES`
- **Structured tracing** with `tracing` + `tracing-subscriber`; optional JSON log format via `JSON_LOGGING=true`

### Added — Frontend (SPA)
- 6-tab dashboard: Encode, Decode, Batch, Payload Builder, History
- Dark / light theme with CSS custom properties; system-preference detection; persisted in `localStorage`
- WebSocket stats chip in header shows live operation counter
- **Camera QR decode** via browser WebRTC (`getUserMedia`) + jsQR (CDN)
- Keyboard shortcuts: `E/D/B/P/H` switch tabs, `Ctrl+Enter` submit, `T` toggle theme, `Esc` clear
- Batch form supports import from JSON file, individual toggles, ZIP download
- Payload Builder: dynamic form rendering for all 10 payload types, "Build + Encode" one-click
- History tab: paginated browsing, thumbnail previews, delete individual / clear all

### Added — DevOps
- Multi-stage `Dockerfile` producing a `< 50 MB` distroless image
- `docker-compose.yml` for one-command local setup
- GitHub Actions `ci.yml`: `fmt`, `clippy`, `test`, `cargo audit`
- GitHub Actions `release.yml`: cross-compile Linux x86_64/aarch64, macOS, Windows; publish to GitHub Releases

### Changed
- Minimum Rust version: **1.82** (required by Axum 0.8 / sqlx 0.8)
- Route parameter syntax updated to Axum 0.8: `{id}` instead of `:id`
- Static file serving changed to `fallback_service` (Axum 0.8 API)
- `BatchEncodeRequest` uses named `defaults: BatchDefaults` field (previously attempted `#[serde(flatten)]`)
- Encode response fields renamed: `image_b64` → `data`, `pixel_size` → `width`/`height`
- Payload type discriminants are `snake_case` (`wifi`, `cal_event`, …)
- `jpeg_quality` type narrowed to `Option<u8>` (was `u32`)

### Removed
- CLI binary target (all functionality now served over HTTP)
- All v1 single-file prototype code

---

## [1.0.0] — 2025-01-01

### Added
- Basic web UI using Axum serving HTML from `static/`
- `POST /encode` — PNG QR generation with foreground/background color
- `POST /decode/upload` — multipart image decode via rxing
- `GET /history` — in-memory history list (no persistence)
- Docker support (single-stage build)

---

## [0.1.0] — 2024-10-15

### Added
- Initial Rust CLI tool: `qrtools encode "text"` outputs QR to terminal (Unicode blocks)
- `qrtools decode <image.png>` — decode PNG via rxing
- Basic `--size`, `--level` flags
