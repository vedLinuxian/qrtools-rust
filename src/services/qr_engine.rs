//! Advanced QR code rendering engine.
//!
//! Provides multi-format output (PNG/JPEG/WebP/SVG), multiple visual styles
//! (square modules, dot/rounded modules, gradient fills), optional logo overlay
//! in the quiet zone, and a frame/badge with custom label text.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use csscolorparser::Color as CssColor;
use image::{
    imageops::{self, FilterType},
    DynamicImage, ImageBuffer, Rgba, RgbaImage,
};
use qrcode::{EcLevel, QrCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ── Public request / response types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Png,
    Jpeg,
    Webp,
    Svg,
}

impl Default for OutputFormat {
    fn default() -> Self { OutputFormat::Png }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleStyle {
    /// Classic filled square (default, compatible with all readers)
    Square,
    /// Circular dots per module (visually distinct, fully scannable)
    Dot,
    /// Rounded-corner squares
    Rounded,
}

impl Default for ModuleStyle {
    fn default() -> Self { ModuleStyle::Square }
}

/// All parameters that control QR code rendering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeOptions {
    /// The data to encode (max 7089 bytes of numeric, 4296 alphanumeric,
    /// 2953 binary / UTF-8 characters).
    #[serde(default)]
    pub data: String,

    #[serde(default)]
    pub format: OutputFormat,

    /// Error-correction level: L(7%) / M(15%) / Q(25%) / H(30%).
    #[serde(default = "default_ec")]
    pub ec_level: String,

    /// Pixel side length of each QR module. Range: 1–50.
    #[serde(default = "default_module_size")]
    pub module_size: u32,

    /// Number of quiet-zone modules on each edge.
    #[serde(default = "default_quiet_zone")]
    pub quiet_zone: u32,

    /// CSS colour for dark modules (e.g. `"#000000"` or `"black"`).
    #[serde(default = "default_fg")]
    pub foreground: String,

    /// CSS colour for light modules / background (e.g. `"#ffffff"`).
    #[serde(default = "default_bg")]
    pub background: String,

    #[serde(default)]
    pub module_style: ModuleStyle,

    // ── SVG gradient ─────────────────────────────────────────────────────────
    /// Optional second CSS colour for a linear gradient on dark modules (SVG only).
    pub gradient_color: Option<String>,

    // ── Logo overlay ──────────────────────────────────────────────────────────
    /// Base64-encoded PNG/JPEG/WebP logo to embed in the QR centre.
    pub logo_base64: Option<String>,

    /// Logo size as a fraction of the QR image (0.1–0.3).  Defaults to 0.22.
    #[serde(default = "default_logo_ratio")]
    pub logo_ratio: f32,

    // ── Badge / frame label ───────────────────────────────────────────────────
    /// Text to add below the QR code inside a badge frame (PNG only).
    pub badge_label: Option<String>,

    // ── JPEG quality ──────────────────────────────────────────────────────────
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,
}

fn default_ec()         -> String { "M".to_string() }
fn default_module_size() -> u32   { 10 }
fn default_quiet_zone() -> u32    { 4 }
fn default_fg()         -> String { "#000000".to_string() }
fn default_bg()         -> String { "#ffffff".to_string() }
fn default_logo_ratio() -> f32    { 0.22 }
fn default_jpeg_quality() -> u8   { 85 }

/// Result of a successful encode operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodeResult {
    pub id: String,
    pub mime_type: String,
    /// Base64-encoded image bytes for PNG/JPEG/WebP; raw SVG string for SVG.
    pub data: String,
    pub width: u32,
    pub height: u32,
    /// Compact 48×48 PNG thumbnail always in base64 (for history display).
    pub thumbnail_b64: String,
}

// ── Public entry point ───────────────────────────────────────────────────────

/// Encode `data` according to `opts`, returning an `EncodeResult`.
pub fn encode(opts: &EncodeOptions) -> Result<EncodeResult, AppError> {
    // Validate
    if opts.data.is_empty() {
        return Err(AppError::InvalidInput("data must not be empty".into()));
    }
    if opts.data.len() > 7089 {
        return Err(AppError::InvalidInput(format!(
            "data too long ({} chars); QR max is 7089 bytes",
            opts.data.len()
        )));
    }
    let ms = opts.module_size.clamp(1, 50);
    let qz = opts.quiet_zone.clamp(0, 10);

    // Parse colours
    let fg = parse_color(&opts.foreground)?;
    let bg = parse_color(&opts.background)?;

    // Build QR matrix
    let ec = parse_ec(&opts.ec_level)?;
    let code = QrCode::with_error_correction_level(opts.data.as_bytes(), ec)
        .map_err(|e| AppError::EncodeError(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();

    match opts.format {
        OutputFormat::Svg => encode_svg(opts, &code, ms, qz, &fg, &bg, &id),
        _ => encode_raster(opts, &code, ms, qz, &fg, &bg, &id),
    }
}

// ── SVG path ─────────────────────────────────────────────────────────────────

fn encode_svg(
    opts: &EncodeOptions,
    code: &QrCode,
    ms: u32,
    qz: u32,
    fg: &[u8; 4],
    bg: &[u8; 4],
    id: &str,
) -> Result<EncodeResult, AppError> {
    let colors = code.to_colors();
    let module_count = (colors.len() as f64).sqrt() as u32;
    let total = module_count + 2 * qz;
    let size = total * ms;

    let fg_hex = rgba_to_hex(fg);
    let bg_hex = rgba_to_hex(bg);

    let mut svg = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}" viewBox="0 0 {s} {s}">
  <defs>"#,
        s = size
    );

    // Optional gradient
    let fill_attr = if let Some(g2) = &opts.gradient_color {
        match parse_color(g2) {
            Ok(gc) => {
                let g2_hex = rgba_to_hex(&gc);
                svg.push_str(&format!(
                    r#"
    <linearGradient id="qrg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="{fg}"/>
      <stop offset="100%" stop-color="{g2}"/>
    </linearGradient>"#,
                    fg = fg_hex,
                    g2 = g2_hex
                ));
                "url(#qrg)".to_string()
            }
            Err(_) => fg_hex.clone(),
        }
    } else {
        fg_hex.clone()
    };

    svg.push_str("\n  </defs>");

    // Background
    svg.push_str(&format!(
        r#"
  <rect width="{s}" height="{s}" fill="{bg}"/>"#,
        s = size,
        bg = bg_hex
    ));

    // Modules
    for (i, color) in colors.iter().enumerate() {
        if *color == qrcode::Color::Dark {
            let col = (i as u32) % module_count;
            let row = (i as u32) / module_count;
            let x = (qz + col) * ms;
            let y = (qz + row) * ms;
            svg.push_str(&module_svg_shape(
                &opts.module_style,
                x, y, ms, &fill_attr,
            ));
        }
    }

    svg.push_str("\n</svg>");

    // Tiny thumbnail from SVG is not trivially possible without resvg; just use
    // the SVG first 48 chars as fallback — store empty for SVG history entries.
    let thumb = build_thumbnail_from_qr(code, qz)?;

    Ok(EncodeResult {
        id: id.to_string(),
        mime_type: "image/svg+xml".into(),
        data: svg,
        width: size,
        height: size,
        thumbnail_b64: thumb,
    })
}

fn module_svg_shape(style: &ModuleStyle, x: u32, y: u32, ms: u32, fill: &str) -> String {
    match style {
        ModuleStyle::Square => format!(
            r#"
  <rect x="{x}" y="{y}" width="{ms}" height="{ms}" fill="{f}"/>"#,
            x = x, y = y, ms = ms, f = fill
        ),
        ModuleStyle::Dot => {
            let r = ms as f32 * 0.45;
            let cx = x as f32 + ms as f32 / 2.0;
            let cy = y as f32 + ms as f32 / 2.0;
            format!(
                r#"
  <circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{f}"/>"#,
                cx = cx, cy = cy, r = r, f = fill
            )
        }
        ModuleStyle::Rounded => {
            let r = ms as f32 * 0.3;
            format!(
                r#"
  <rect x="{x}" y="{y}" width="{ms}" height="{ms}" rx="{r:.1}" ry="{r:.1}" fill="{f}"/>"#,
                x = x, y = y, ms = ms, r = r, f = fill
            )
        }
    }
}

// ── Raster path (PNG / JPEG / WebP) ──────────────────────────────────────────

fn encode_raster(
    opts: &EncodeOptions,
    code: &QrCode,
    ms: u32,
    qz: u32,
    fg: &[u8; 4],
    bg: &[u8; 4],
    id: &str,
) -> Result<EncodeResult, AppError> {
    let colors = code.to_colors();
    let module_count = (colors.len() as f64).sqrt() as u32;
    let total = module_count + 2 * qz;
    let size = total * ms;

    let mut img: RgbaImage = ImageBuffer::new(size, size);

    // Fill background
    for pixel in img.pixels_mut() {
        *pixel = Rgba(*bg);
    }

    // Draw modules
    let dot_mode = matches!(opts.module_style, ModuleStyle::Dot);
    let rounded_mode = matches!(opts.module_style, ModuleStyle::Rounded);

    for (i, color) in colors.iter().enumerate() {
        if *color == qrcode::Color::Dark {
            let col = (i as u32) % module_count;
            let row = (i as u32) / module_count;
            let ox = (qz + col) * ms;
            let oy = (qz + row) * ms;

            if dot_mode {
                draw_circle(&mut img, ox, oy, ms, fg);
            } else if rounded_mode {
                draw_rounded_rect(&mut img, ox, oy, ms, fg);
            } else {
                // Square (fast path)
                for dy in 0..ms {
                    for dx in 0..ms {
                        img.put_pixel(ox + dx, oy + dy, Rgba(*fg));
                    }
                }
            }
        }
    }

    // Overlay logo
    if let Some(logo_b64) = &opts.logo_base64 {
        if let Ok(logo_bytes) = B64.decode(logo_b64) {
            if let Ok(logo_img) = image::load_from_memory(&logo_bytes) {
                let ratio = opts.logo_ratio.clamp(0.1, 0.3);
                let logo_size = (size as f32 * ratio) as u32;
                let logo_resized = logo_img.resize(logo_size, logo_size, FilterType::Lanczos3);
                let lx = (size - logo_size) / 2;
                let ly = (size - logo_size) / 2;
                imageops::overlay(&mut img, &logo_resized.to_rgba8(), lx as i64, ly as i64);
            }
        }
    }

    // Badge label
    let img = if let Some(label) = &opts.badge_label {
        add_badge_label(img, label, bg, fg)?
    } else {
        img
    };

    let (width, height) = (img.width(), img.height());

    // Encode to requested format
    let dyn_img = DynamicImage::ImageRgba8(img);
    let (encoded, mime_type) = encode_image_format(&dyn_img, &opts.format, opts.jpeg_quality)?;
    let data = B64.encode(&encoded);

    // Build 48×48 thumbnail
    let thumb_img = dyn_img.resize(48, 48, FilterType::Triangle);
    let mut thumb_buf = Vec::new();
    thumb_img
        .write_to(
            &mut std::io::Cursor::new(&mut thumb_buf),
            image::ImageFormat::Png,
        )
        .map_err(|e| AppError::EncodeError(e.to_string()))?;
    let thumbnail_b64 = B64.encode(&thumb_buf);

    Ok(EncodeResult {
        id: id.to_string(),
        mime_type: mime_type.to_string(),
        data,
        width,
        height,
        thumbnail_b64,
    })
}

fn encode_image_format(
    img: &DynamicImage,
    fmt: &OutputFormat,
    jpeg_quality: u8,
) -> Result<(Vec<u8>, &'static str), AppError> {
    let mut buf = Vec::new();
    match fmt {
        OutputFormat::Jpeg => {
            let rgb = img.to_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut buf, jpeg_quality,
            );
            encoder
                .encode_image(&DynamicImage::ImageRgb8(rgb))
                .map_err(|e| AppError::EncodeError(e.to_string()))?;
            Ok((buf, "image/jpeg"))
        }
        OutputFormat::Webp => {
            img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP)
                .map_err(|e| AppError::EncodeError(e.to_string()))?;
            Ok((buf, "image/webp"))
        }
        _ => {
            // PNG (default + fallback)
            img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| AppError::EncodeError(e.to_string()))?;
            Ok((buf, "image/png"))
        }
    }
}

// ── Drawing helpers ───────────────────────────────────────────────────────────

fn draw_circle(img: &mut RgbaImage, ox: u32, oy: u32, ms: u32, color: &[u8; 4]) {
    let r = ms as f32 / 2.0;
    let cx = ox as f32 + r;
    let cy = oy as f32 + r;
    let r_sq = (r * 0.93) * (r * 0.93);
    for dy in 0..ms {
        for dx in 0..ms {
            let px = ox + dx;
            let py = oy + dy;
            let fx = px as f32 + 0.5 - cx;
            let fy = py as f32 + 0.5 - cy;
            if fx * fx + fy * fy <= r_sq {
                img.put_pixel(px, py, Rgba(*color));
            }
        }
    }
}

fn draw_rounded_rect(img: &mut RgbaImage, ox: u32, oy: u32, ms: u32, color: &[u8; 4]) {
    let r = (ms as f32 * 0.28).max(1.0);
    for dy in 0..ms {
        for dx in 0..ms {
            let px = ox + dx;
            let py = oy + dy;
            let fx = dx as f32 + 0.5;
            let fy = dy as f32 + 0.5;
            let ms_f = ms as f32;
            // Distance to nearest corner
            let corner_x = if fx < r { r } else if fx > ms_f - r { ms_f - r } else { fx };
            let corner_y = if fy < r { r } else if fy > ms_f - r { ms_f - r } else { fy };
            let dist = ((fx - corner_x).powi(2) + (fy - corner_y).powi(2)).sqrt();
            if dist <= r || (fx >= r && fx <= ms_f - r) || (fy >= r && fy <= ms_f - r) {
                if fx >= 0.0 && fx < ms_f && fy >= 0.0 && fy < ms_f {
                    img.put_pixel(px, py, Rgba(*color));
                }
            }
        }
    }
}

fn add_badge_label(
    qr_img: RgbaImage,
    label: &str,
    bg: &[u8; 4],
    _fg: &[u8; 4],
) -> Result<RgbaImage, AppError> {
    let badge_h = (qr_img.height() / 8).max(24);
    let total_h = qr_img.height() + badge_h;
    let w = qr_img.width();

    let mut canvas: RgbaImage = ImageBuffer::new(w, total_h);

    // Draw background
    for pixel in canvas.pixels_mut() {
        *pixel = Rgba(*bg);
    }

    // Copy QR on top
    imageops::overlay(&mut canvas, &qr_img, 0, 0);

    // Draw badge background stripe (slightly darker)
    let stripe_color = Rgba([
        bg[0].saturating_sub(20),
        bg[1].saturating_sub(20),
        bg[2].saturating_sub(20),
        bg[3],
    ]);
    for y in qr_img.height()..total_h {
        for x in 0..w {
            canvas.put_pixel(x, y, stripe_color);
        }
    }

    // Use imageproc to draw label text
    let font_bytes = include_bytes!("../../assets/DejaVuSans.ttf");
    let font = ab_glyph::FontRef::try_from_slice(font_bytes)
        .map_err(|e| AppError::EncodeError(format!("font load: {e}")))?;
    let scale = ab_glyph::PxScale::from((badge_h as f32 * 0.55).max(12.0));
    let text_color = image::Rgba([30u8, 30, 30, 255]);
    let text_w = imageproc::drawing::text_size(scale, &font, label).0;
    let tx = ((w as i32) - text_w as i32).max(0) / 2;
    let ty = qr_img.height() as i32 + (badge_h as i32 - scale.y as i32) / 2;
    imageproc::drawing::draw_text_mut(&mut canvas, text_color, tx, ty, scale, &font, label);

    Ok(canvas)
}

// ── Thumbnail from QR code matrix (for SVG entries) ──────────────────────────

fn build_thumbnail_from_qr(code: &QrCode, qz: u32) -> Result<String, AppError> {
    let colors = code.to_colors();
    let module_count = (colors.len() as f64).sqrt() as u32;
    let ms = 2u32;
    let total = module_count + 2 * qz;
    let size = (total * ms).min(96);
    let ms_small = if total > 0 { size / total } else { 1 };

    let mut img: RgbaImage = ImageBuffer::new(size, size);
    for pixel in img.pixels_mut() {
        *pixel = Rgba([255, 255, 255, 255]);
    }
    for (i, color) in colors.iter().enumerate() {
        if *color == qrcode::Color::Dark {
            let col = (i as u32) % module_count;
            let row = (i as u32) / module_count;
            let ox = (qz + col) * ms_small;
            let oy = (qz + row) * ms_small;
            for dy in 0..ms_small {
                for dx in 0..ms_small {
                    if ox + dx < size && oy + dy < size {
                        img.put_pixel(ox + dx, oy + dy, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }
    let mut buf = Vec::new();
    DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| AppError::EncodeError(e.to_string()))?;
    Ok(B64.encode(&buf))
}

// ── Utilities ─────────────────────────────────────────────────────────────────

pub fn parse_ec(s: &str) -> Result<EcLevel, AppError> {
    match s.to_uppercase().as_str() {
        "L" => Ok(EcLevel::L),
        "M" => Ok(EcLevel::M),
        "Q" => Ok(EcLevel::Q),
        "H" => Ok(EcLevel::H),
        other => Err(AppError::InvalidInput(format!(
            "unknown EC level '{other}'; expected L/M/Q/H"
        ))),
    }
}

fn parse_color(s: &str) -> Result<[u8; 4], AppError> {
    let c: CssColor = s
        .parse()
        .map_err(|_| AppError::InvalidInput(format!("invalid CSS color: '{s}'")))?;
    Ok([
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * 255.0).round() as u8,
    ])
}

fn rgba_to_hex(c: &[u8; 4]) -> String {
    if c[3] == 255 {
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("#{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}
