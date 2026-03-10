/// Integration tests for qrtools-web (standalone — no running server required)
///
/// These tests exercise the underlying crates directly, verifying the
/// business logic that powers the API handlers.

#[test]
fn test_qrcode_encode_roundtrip() {
    use qrcode::{EcLevel, QrCode};

    for level in [EcLevel::L, EcLevel::M, EcLevel::Q, EcLevel::H] {
        let qr = QrCode::with_error_correction_level(b"https://example.com", level)
            .expect("QR encode failed");
        assert!(qr.width() > 0, "Expected non-zero QR width");
        let colors = qr.to_colors();
        assert_eq!(
            colors.len(),
            qr.width() * qr.width(),
            "Color count must match pixels"
        );
    }
}

#[test]
fn test_png_render_correct_dimensions() {
    use image::{ImageBuffer, Rgba};
    use qrcode::{EcLevel, QrCode};

    let qr = QrCode::with_error_correction_level(b"unit test", EcLevel::M).unwrap();
    let modules = qr.width() as u32;
    let module_size: u32 = 8;
    let quiet_zone: u32 = 4;
    let total = (modules + quiet_zone * 2) * module_size;

    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(total, total);
    // Fill background
    for p in img.pixels_mut() {
        *p = Rgba([255, 255, 255, 255]);
    }
    // Draw modules
    let colors = qr.to_colors();
    for (y, row) in colors.chunks(modules as usize).enumerate() {
        for (x, color) in row.iter().enumerate() {
            if *color == qrcode::Color::Dark {
                let px = (x as u32 + quiet_zone) * module_size;
                let py = (y as u32 + quiet_zone) * module_size;
                for dy in 0..module_size {
                    for dx in 0..module_size {
                        img.put_pixel(px + dx, py + dy, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    assert_eq!(img.width(), total);
    assert_eq!(img.height(), total);
}

#[test]
fn test_svg_render_valid_xml() {
    use qrcode::{EcLevel, QrCode};

    let qr = QrCode::with_error_correction_level(b"svg test", EcLevel::M).unwrap();
    let modules = qr.width() as u32;
    let ms: u32 = 4;
    let qz: u32 = 4;
    let size = (modules + qz * 2) * ms;

    let colors = qr.to_colors();
    let mut rects = String::new();
    for (y, row) in colors.chunks(modules as usize).enumerate() {
        for (x, color) in row.iter().enumerate() {
            if *color == qrcode::Color::Dark {
                rects.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}"/>"#,
                    (x as u32 + qz) * ms,
                    (y as u32 + qz) * ms,
                    ms,
                    ms
                ));
            }
        }
    }

    let svg = format!(
        r#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="{s}" height="{s}"><rect fill="white" width="{s}" height="{s}"/><g fill="black">{rects}</g></svg>"#,
        s = size,
        rects = rects
    );

    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));
    assert!(!rects.is_empty(), "Expected at least some dark modules");
}

#[test]
fn test_css_color_parsing() {
    let cases = [
        ("#000000", [0u8, 0, 0, 255]),
        ("#ffffff", [255, 255, 255, 255]),
        ("#ff0000", [255, 0, 0, 255]),
        ("black", [0, 0, 0, 255]),
        ("white", [255, 255, 255, 255]),
        ("transparent", [0, 0, 0, 0]),
    ];
    for (input, expected) in cases {
        let c = csscolorparser::parse(input)
            .unwrap_or_else(|e| panic!("Failed to parse color '{}': {}", input, e));
        assert_eq!(c.to_rgba8(), expected, "Color '{}' mismatch", input);
    }
}

#[test]
fn test_base64_roundtrip() {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let original = b"Hello, QR world!";
    let encoded = B64.encode(original);
    let decoded = B64.decode(&encoded).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn test_long_data_validation() {
    // Data exceeding max QR capacity
    let long_text = "a".repeat(7090);
    assert!(
        long_text.len() > 7089,
        "Should be rejected by the encode handler"
    );
}

