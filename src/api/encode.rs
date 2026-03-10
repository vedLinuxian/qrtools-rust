use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use image::{ImageBuffer, Rgba};
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    state::{AppState, HistoryEntry, OperationKind},
};

/// Request body for the encode endpoint.
#[derive(Debug, Deserialize)]
pub struct EncodeRequest {
    /// The text or URL to encode
    pub data: String,
    /// Output format: "png" (default) | "svg"
    #[serde(default = "default_format")]
    pub format: OutputFormat,
    /// Error correction level: "L" | "M" (default) | "Q" | "H"
    #[serde(default = "default_ec_level")]
    pub ec_level: EcLevelInput,
    /// QR module size in pixels (for PNG, default 10)
    #[serde(default = "default_module_size")]
    pub module_size: u32,
    /// Quiet zone (border) in modules (default 4)
    #[serde(default = "default_quiet_zone")]
    pub quiet_zone: u32,
    /// Foreground color as CSS color string (default "#000000")
    #[serde(default = "default_fg")]
    pub foreground: String,
    /// Background color as CSS color string (default "#ffffff")
    #[serde(default = "default_bg")]
    pub background: String,
}

fn default_format() -> OutputFormat {
    OutputFormat::Png
}
fn default_ec_level() -> EcLevelInput {
    EcLevelInput::M
}
fn default_module_size() -> u32 {
    10
}
fn default_quiet_zone() -> u32 {
    4
}
fn default_fg() -> String {
    "#000000".to_string()
}
fn default_bg() -> String {
    "#ffffff".to_string()
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Svg,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum EcLevelInput {
    L,
    M,
    Q,
    H,
}

impl From<EcLevelInput> for EcLevel {
    fn from(e: EcLevelInput) -> Self {
        match e {
            EcLevelInput::L => EcLevel::L,
            EcLevelInput::M => EcLevel::M,
            EcLevelInput::Q => EcLevel::Q,
            EcLevelInput::H => EcLevel::H,
        }
    }
}

fn parse_css_color(s: &str) -> Result<[u8; 4], AppError> {
    let c = csscolorparser::parse(s)
        .map_err(|e| AppError::InvalidInput(format!("Invalid color '{}': {}", s, e)))?;
    let [r, g, b, a] = c.to_rgba8();
    Ok([r, g, b, a])
}

/// Response body for the encode endpoint.
#[derive(Debug, Serialize)]
pub struct EncodeResponse {
    /// Unique operation ID
    pub id: String,
    /// MIME type of the encoded output ("image/png" or "image/svg+xml")
    pub mime_type: String,
    /// Base64-encoded content (PNG) or raw SVG string
    pub data: String,
    /// Width of the generated image in pixels (PNG only)
    pub width: Option<u32>,
    /// Height of the generated image in pixels (PNG only)
    pub height: Option<u32>,
    /// ISO 8601 timestamp
    pub timestamp: String,
}

/// POST /api/encode
pub async fn encode(
    State(state): State<AppState>,
    Json(req): Json<EncodeRequest>,
) -> Result<Json<EncodeResponse>, AppError> {
    // Validate request
    if req.data.is_empty() {
        return Err(AppError::InvalidInput(
            "Field 'data' must not be empty".to_string(),
        ));
    }
    if req.data.len() > 7089 {
        return Err(AppError::InvalidInput(format!(
            "Data too long ({} chars); QR code maximum is 7089 numeric characters",
            req.data.len()
        )));
    }
    if req.module_size == 0 || req.module_size > 50 {
        return Err(AppError::InvalidInput(
            "module_size must be between 1 and 50".to_string(),
        ));
    }

    let ec = EcLevel::from(req.ec_level.clone());
    let qr = QrCode::with_error_correction_level(req.data.as_bytes(), ec)
        .map_err(|e| AppError::EncodeError(e.to_string()))?;

    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    state.increment_ops();

    match req.format {
        OutputFormat::Png => {
            let fg = parse_css_color(&req.foreground)?;
            let bg = parse_css_color(&req.background)?;

            let qz = req.quiet_zone;
            let ms = req.module_size;

            // Get module grid
            let width_modules = qr.width() as u32;
            let total_modules = width_modules + qz * 2;
            let px = total_modules * ms;

            let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(px, px);

            // Fill background
            for pixel in img.pixels_mut() {
                *pixel = Rgba(bg);
            }

            // Draw modules
            let image_data = qr.to_colors();
            for (y, row) in image_data.chunks(width_modules as usize).enumerate() {
                for (x, color) in row.iter().enumerate() {
                    if *color == qrcode::Color::Dark {
                        let px_x = (x as u32 + qz) * ms;
                        let px_y = (y as u32 + qz) * ms;
                        for dy in 0..ms {
                            for dx in 0..ms {
                                img.put_pixel(px_x + dx, px_y + dy, Rgba(fg));
                            }
                        }
                    }
                }
            }

            // Encode to PNG bytes
            let mut png_bytes: Vec<u8> = Vec::new();
            img.write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )?;

            let encoded = BASE64.encode(&png_bytes);

            // Store in history
            let preview = BASE64.encode(&png_bytes);
            let entry = HistoryEntry {
                id: id.clone(),
                kind: OperationKind::Encode,
                summary: truncate(&req.data, 60),
                timestamp: Utc::now(),
                preview_png_b64: Some(preview),
            };
            state.history.insert(id.clone(), entry).await;

            Ok(Json(EncodeResponse {
                id,
                mime_type: "image/png".to_string(),
                data: encoded,
                width: Some(px),
                height: Some(px),
                timestamp,
            }))
        }

        OutputFormat::Svg => {
            let fg = req.foreground.clone();
            let bg = req.background.clone();
            let qz = req.quiet_zone;
            let ms = req.module_size;

            let width_modules = qr.width() as u32;
            let total_modules = width_modules + qz * 2;
            let svg_size = total_modules * ms;

            let image_data = qr.to_colors();
            let mut rects = String::new();
            for (y, row) in image_data.chunks(width_modules as usize).enumerate() {
                for (x, color) in row.iter().enumerate() {
                    if *color == qrcode::Color::Dark {
                        let px_x = (x as u32 + qz) * ms;
                        let px_y = (y as u32 + qz) * ms;
                        rects.push_str(&format!(
                            r#"<rect x="{}" y="{}" width="{}" height="{}"/>"#,
                            px_x, px_y, ms, ms
                        ));
                    }
                }
            }

            let svg = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1" viewBox="0 0 {size} {size}" width="{size}" height="{size}">
  <rect x="0" y="0" width="{size}" height="{size}" fill="{bg}"/>
  <g fill="{fg}">{rects}</g>
</svg>"#,
                size = svg_size,
                bg = bg,
                fg = fg,
                rects = rects
            );

            let entry = HistoryEntry {
                id: id.clone(),
                kind: OperationKind::Encode,
                summary: truncate(&req.data, 60),
                timestamp: Utc::now(),
                preview_png_b64: None,
            };
            state.history.insert(id.clone(), entry).await;

            Ok(Json(EncodeResponse {
                id,
                mime_type: "image/svg+xml".to_string(),
                data: svg,
                width: Some(svg_size),
                height: Some(svg_size),
                timestamp,
            }))
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}
