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

/// Build the invisible text content stream.
///
/// Uses render mode 3 (Tr 3) so glyphs are drawn but invisible, enabling
/// text selection and search without affecting visual appearance.
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

        // Text matrix: diagonal sets effective font size (Tf size=1, so h acts as font size).
        // No horizontal scaling for POC — Tz defaults to 100%.
        out.extend_from_slice(
            format!(
                "{:.4} 0 0 {:.4} {:.4} {:.4} Tm\n",
                tb.height, tb.height, tb.x, tb.y
            )
            .as_bytes(),
        );

        // Encode as WinAnsiEncoding (Latin-1 chars map directly by codepoint value).
        let encoded: Vec<u8> = trimmed
            .chars()
            .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
            .collect();

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
