# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Axum 0.8 REST API** — full async HTTP server with typed extractors
- **Encode endpoint** (`POST /api/encode`) — generates QR codes as PNG or SVG with configurable:
  - Error correction level (L / M / Q / H)
  - Module size (1–50 px)
  - Foreground and background colour (any CSS colour string)
  - Output format (PNG → base64, SVG → raw XML)
- **Decode endpoints**:
  - `POST /api/decode/json` — decode QR from base64-encoded image payload
  - `POST /api/decode/upload` — decode QR from multipart/form-data file upload
- **History endpoint** (`GET /api/history`, `DELETE /api/history`) — async LRU cache (moka 0.12) stores last 100 encode/decode operations with timestamps
- **Health endpoints** (`GET /api/health`, `GET /api/health/ready`) — liveness + readiness probes
- **4-tab SPA dashboard** (`static/index.html`)
  - **Encode tab** — live QR preview, download PNG/SVG, copy to clipboard
  - **Decode tab** — drag-and-drop image upload, base64 paste
  - **Batch tab** — encode up to 50 QR codes in a single session
  - **History tab** — view and clear recent operations
- **Dark / light theme** — CSS custom properties, system-preference-aware, manual toggle
- **Configuration via environment variables** (see `.env.example`):
  - `PORT`, `HOST`, `MAX_UPLOAD_BYTES`, `RUST_LOG`, `CORS_ALLOWED_ORIGIN`, `CACHE_CAPACITY`
- **Multi-stage Docker image** (~50 MB) + `docker-compose.yml`
- **GitHub Actions CI** — lint (`fmt` + `clippy`), test, `cargo-audit`, Docker build on push to `main`
- **Release workflow** — cross-compile matrix: linux/macOS/Windows × x86_64/aarch64; auto-publishes GitHub Release with binary artifacts
- **Integration tests** — 6 standalone tests covering encode roundtrip, PNG dimensions, SVG validity, CSS colour parsing, base64 roundtrip, data-length validation
- **GitHub Pages homepage** at <https://vedlinuxian.github.io/qrtools-rust/>

### Changed
- Replaced original CLI tool (`qrtool`) architecture with an async web service
- Updated all dependencies to 2025/2026 stable releases:
  - `axum 0.8`, `tokio 1`, `tower-http 0.6`, `qrcode 0.14`, `rxing 0.6`, `image 0.25`, `moka 0.12`, `serde 1`, `base64 0.22`, `uuid 1`, `chrono 0.4`, `thiserror 2`, `anyhow 1`
- Middleware stack moved from `ServiceBuilder` to chained `.layer()` calls on the router (avoids `ResponseBody: Default` trait bound conflict in tower-http 0.6)

### Fixed
- `rxing::helpers::detect_in_luma` now called with correct 4-argument signature (added `None` for `Option<BarcodeFormat>` parameter)
- Removed non-existent `rxing` Cargo features (`allow_product_codes`, `allow_2d_codes`) that caused build failures
- `TimeoutLayer::with_status_code` argument order corrected (`StatusCode` first, then `Duration`)
- Removed unused imports (`Luma`, `Version`) that produced compiler warnings
- Integration tests rewritten as top-level functions (binary-only crate has no lib target, cannot use `crate::` imports in test files)

### Security
- `cargo-audit` job in CI blocks merges if any advisory is found in the dependency tree
- `RequestBodyLimitLayer` caps upload payload at configurable `MAX_UPLOAD_BYTES` (default: 10 MB)
- CORS origin restricted via `CORS_ALLOWED_ORIGIN` env var (defaults to all in dev, should be tightened in production)
- Docker runtime uses `gcr.io/distroless/cc-debian12` — no shell, no package manager, minimal attack surface

## [0.1.0] — _initial release_

- Original `qrtool` CLI codebase (encode/decode QR codes via command line)
