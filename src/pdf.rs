use anyhow::{anyhow, Result};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::model::Orientation;
use crate::transform::TokenBox;

/// Returns (width_pts, height_pts) for the given page by walking the page tree to find MediaBox.
pub fn get_page_dimensions(doc: &Document, page_id: ObjectId) -> Result<(f64, f64)> {
    let mut current = page_id;
    loop {
        let dict = doc
            .get_object(current)
            .map_err(|e| anyhow!("get_object({:?}): {}", current, e))?
            .as_dict()
            .map_err(|e| anyhow!("as_dict: {}", e))?
            .clone();

        if let Ok(mb) = dict.get(b"MediaBox") {
            let arr = mb.as_array().map_err(|e| anyhow!("MediaBox as_array: {}", e))?;
            if arr.len() < 4 {
                return Err(anyhow!("MediaBox has {} elements, need 4", arr.len()));
            }
            let vals: Vec<f64> = arr.iter().map(obj_to_f64).collect::<Result<_>>()?;
            return Ok((vals[2] - vals[0], vals[3] - vals[1]));
        }

        let parent = dict
            .get(b"Parent")
            .map_err(|_| anyhow!("no MediaBox and no Parent in page tree"))?
            .as_reference()
            .map_err(|e| anyhow!("Parent as_reference: {}", e))?;
        current = parent;
    }
}

fn obj_to_f64(o: &Object) -> Result<f64> {
    match o {
        Object::Integer(n) => Ok(*n as f64),
        Object::Real(r) => Ok(*r as f64),
        _ => Err(anyhow!("expected numeric PDF object")),
    }
}

/// Helvetica advance widths (WinAnsiEncoding) in units per 1000 em.
/// Source: Adobe Helvetica AFM file.
fn helvetica_width(byte: u8) -> u32 {
    match byte {
        0x20 => 278, // space
        0x21 => 278, // !
        0x22 => 355, // "
        0x23 => 556, // #
        0x24 => 556, // $
        0x25 => 889, // %
        0x26 => 667, // &
        0x27 => 191, // '
        0x28 => 333, // (
        0x29 => 333, // )
        0x2A => 389, // *
        0x2B => 584, // +
        0x2C => 278, // ,
        0x2D => 333, // -
        0x2E => 278, // .
        0x2F => 278, // /
        0x30..=0x39 => 556, // 0–9
        0x3A => 278, // :
        0x3B => 278, // ;
        0x3C => 584, // <
        0x3D => 584, // =
        0x3E => 584, // >
        0x3F => 556, // ?
        0x40 => 1015, // @
        0x41 => 667,  // A
        0x42 => 667,  // B
        0x43 => 722,  // C
        0x44 => 722,  // D
        0x45 => 667,  // E
        0x46 => 611,  // F
        0x47 => 778,  // G
        0x48 => 722,  // H
        0x49 => 278,  // I
        0x4A => 500,  // J
        0x4B => 667,  // K
        0x4C => 556,  // L
        0x4D => 833,  // M
        0x4E => 722,  // N
        0x4F => 778,  // O
        0x50 => 667,  // P
        0x51 => 778,  // Q
        0x52 => 722,  // R
        0x53 => 667,  // S
        0x54 => 611,  // T
        0x55 => 722,  // U
        0x56 => 667,  // V
        0x57 => 944,  // W
        0x58 => 667,  // X
        0x59 => 667,  // Y
        0x5A => 611,  // Z
        0x5B => 278,  // [
        0x5C => 278,  // \
        0x5D => 278,  // ]
        0x5E => 469,  // ^
        0x5F => 556,  // _
        0x60 => 333,  // `
        0x61 => 556,  // a
        0x62 => 556,  // b
        0x63 => 500,  // c
        0x64 => 556,  // d
        0x65 => 556,  // e
        0x66 => 278,  // f
        0x67 => 556,  // g
        0x68 => 556,  // h
        0x69 => 222,  // i
        0x6A => 222,  // j
        0x6B => 500,  // k
        0x6C => 222,  // l
        0x6D => 833,  // m
        0x6E => 556,  // n
        0x6F => 556,  // o
        0x70 => 556,  // p
        0x71 => 556,  // q
        0x72 => 333,  // r
        0x73 => 500,  // s
        0x74 => 278,  // t
        0x75 => 556,  // u
        0x76 => 500,  // v
        0x77 => 722,  // w
        0x78 => 500,  // x
        0x79 => 500,  // y
        0x7A => 500,  // z
        0x7B => 334,  // {
        0x7C => 260,  // |
        0x7D => 334,  // }
        0x7E => 584,  // ~
        0xA0 => 278,  // non-breaking space
        0xA1 => 333,  // ¡
        0xA2 => 556,  // ¢
        0xA3 => 556,  // £
        0xA4 => 556,  // ¤
        0xA5 => 556,  // ¥
        0xA6 => 260,  // ¦
        0xA7 => 556,  // §
        0xA8 => 333,  // ¨
        0xA9 => 737,  // ©
        0xAA => 370,  // ª
        0xAB => 556,  // «
        0xAC => 584,  // ¬
        0xAD => 333,  // soft hyphen
        0xAE => 737,  // ®
        0xAF => 333,  // ¯
        0xB0 => 400,  // °
        0xB1 => 584,  // ±
        0xB2 => 333,  // ²
        0xB3 => 333,  // ³
        0xB4 => 333,  // ´
        0xB5 => 556,  // µ
        0xB6 => 537,  // ¶
        0xB7 => 278,  // ·
        0xB8 => 333,  // ¸
        0xB9 => 333,  // ¹
        0xBA => 365,  // º
        0xBB => 556,  // »
        0xBC => 834,  // ¼
        0xBD => 834,  // ½
        0xBE => 834,  // ¾
        0xBF => 611,  // ¿
        0xC0 => 667,  // À
        0xC1 => 667,  // Á
        0xC2 => 667,  // Â
        0xC3 => 667,  // Ã
        0xC4 => 667,  // Ä
        0xC5 => 667,  // Å
        0xC6 => 1000, // Æ
        0xC7 => 722,  // Ç
        0xC8 => 667,  // È
        0xC9 => 667,  // É
        0xCA => 667,  // Ê
        0xCB => 667,  // Ë
        0xCC => 278,  // Ì
        0xCD => 278,  // Í
        0xCE => 278,  // Î
        0xCF => 278,  // Ï
        0xD0 => 722,  // Ð
        0xD1 => 722,  // Ñ
        0xD2 => 778,  // Ò
        0xD3 => 778,  // Ó
        0xD4 => 778,  // Ô
        0xD5 => 778,  // Õ
        0xD6 => 778,  // Ö
        0xD7 => 584,  // ×
        0xD8 => 778,  // Ø
        0xD9 => 722,  // Ù
        0xDA => 722,  // Ú
        0xDB => 722,  // Û
        0xDC => 722,  // Ü
        0xDD => 667,  // Ý
        0xDE => 667,  // Þ
        0xDF => 611,  // ß
        0xE0 => 556,  // à
        0xE1 => 556,  // á
        0xE2 => 556,  // â
        0xE3 => 556,  // ã
        0xE4 => 556,  // ä
        0xE5 => 556,  // å
        0xE6 => 889,  // æ
        0xE7 => 500,  // ç
        0xE8 => 556,  // è
        0xE9 => 556,  // é
        0xEA => 556,  // ê
        0xEB => 556,  // ë
        0xEC => 278,  // ì
        0xED => 278,  // í
        0xEE => 278,  // î
        0xEF => 278,  // ï
        0xF0 => 556,  // ð
        0xF1 => 556,  // ñ
        0xF2 => 556,  // ò
        0xF3 => 556,  // ó
        0xF4 => 556,  // ô
        0xF5 => 556,  // õ
        0xF6 => 556,  // ö
        0xF7 => 584,  // ÷
        0xF8 => 611,  // ø
        0xF9 => 556,  // ù
        0xFA => 556,  // ú
        0xFB => 556,  // û
        0xFC => 556,  // ü
        0xFD => 500,  // ý
        0xFE => 556,  // þ
        0xFF => 500,  // ÿ
        _ => 556,     // fallback for unmapped bytes
    }
}

/// Build the invisible text content stream.
///
/// Uses render mode 3 (Tr 3) so glyphs are drawn but invisible, enabling
/// text selection and search without affecting visual appearance.
/// Tz (horizontal scaling) is set per token so each token's glyphs fill
/// its bounding box width exactly.
pub fn build_text_stream(tokens: &[(TokenBox, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BT\n/OCR_F1 1 Tf\n3 Tr\n");

    for (tb, text) in tokens {
        if tb.orientation != Orientation::PageUp {
            continue;
        }
        let trimmed = text.trim_end();
        if trimmed.is_empty() || tb.height < 0.001 || tb.width < 0.001 {
            continue;
        }

        // Encode as WinAnsiEncoding (Latin-1 chars map directly by codepoint value).
        let encoded: Vec<u8> = trimmed
            .chars()
            .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
            .collect();

        // Horizontal scale: stretch glyphs to fill the token bounding box width.
        // natural_width = sum_of_advances * font_size / 1000  (font_size = tb.height since Tf=1)
        let sum_advances: f64 = encoded.iter().map(|&b| helvetica_width(b) as f64).sum();
        let natural_width = sum_advances * tb.height / 1000.0;
        let tz_percent = if natural_width > 0.001 {
            (tb.width / natural_width * 100.0).clamp(10.0, 1000.0)
        } else {
            100.0
        };

        // Text matrix: diagonal sets effective font size (Tf size=1, so h acts as font size).
        out.extend_from_slice(
            format!(
                "{:.4} 0 0 {:.4} {:.4} {:.4} Tm\n",
                tb.height, tb.height, tb.x, tb.y
            )
            .as_bytes(),
        );

        // Per-token horizontal scale to fit bounding box width.
        out.extend_from_slice(format!("{:.4} Tz\n", tz_percent).as_bytes());

        out.push(b'<');
        for byte in &encoded {
            out.extend_from_slice(format!("{byte:02X}").as_bytes());
        }
        out.extend_from_slice(b"> Tj\n");
    }

    out.extend_from_slice(b"ET\n");
    out
}

/// Append an invisible text overlay stream to `page_id` and register the font.
pub fn add_text_overlay(
    doc: &mut Document,
    page_id: ObjectId,
    font_id: ObjectId,
    content_bytes: Vec<u8>,
) -> Result<()> {
    // Add the text content stream as a new PDF object.
    let stream_id = doc.add_object(Object::Stream(Stream::new(
        Dictionary::new(),
        content_bytes,
    )));

    // Clone the page dict so we can modify it without conflicting borrows.
    let mut page_dict = doc
        .get_object(page_id)
        .map_err(|e| anyhow!("get page: {}", e))?
        .as_dict()
        .map_err(|e| anyhow!("page as_dict: {}", e))?
        .clone();

    // Append stream to /Contents.
    let new_contents = match page_dict.get(b"Contents").ok().cloned() {
        Some(Object::Reference(id)) => Object::Array(vec![
            Object::Reference(id),
            Object::Reference(stream_id),
        ]),
        Some(Object::Array(mut arr)) => {
            arr.push(Object::Reference(stream_id));
            Object::Array(arr)
        }
        _ => Object::Reference(stream_id),
    };
    page_dict.set(b"Contents", new_contents);

    // Inject the font into /Resources/Font.
    // Resources may be an indirect object or inline in the page dict.
    let res_id: Option<ObjectId> = match page_dict.get(b"Resources").ok() {
        Some(Object::Reference(id)) => Some(*id),
        _ => None,
    };

    if let Some(id) = res_id {
        let mut res_dict = doc
            .get_object(id)
            .map_err(|e| anyhow!("get resources: {}", e))?
            .as_dict()
            .map_err(|e| anyhow!("resources as_dict: {}", e))?
            .clone();
        inject_font(&mut res_dict, font_id);
        if let Some(obj) = doc.objects.get_mut(&id) {
            *obj = Object::Dictionary(res_dict);
        }
    } else {
        let mut res_dict = match page_dict.get(b"Resources").ok() {
            Some(Object::Dictionary(d)) => d.clone(),
            _ => Dictionary::new(),
        };
        inject_font(&mut res_dict, font_id);
        page_dict.set(b"Resources", Object::Dictionary(res_dict));
    }

    // Write the modified page dict back.
    if let Some(obj) = doc.objects.get_mut(&page_id) {
        *obj = Object::Dictionary(page_dict);
    } else {
        return Err(anyhow!("page {:?} not found in document objects", page_id));
    }

    Ok(())
}

fn inject_font(resources: &mut Dictionary, font_id: ObjectId) {
    let mut font_dict = match resources.get(b"Font").ok() {
        Some(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };
    font_dict.set(b"OCR_F1", Object::Reference(font_id));
    resources.set(b"Font", Object::Dictionary(font_dict));
}
