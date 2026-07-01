use anyhow::anyhow;

use crate::model::{NormalizedVertex, Orientation};

/// A token's bounding box in PDF coordinate space (points, bottom-left origin).
#[derive(Debug, Clone)]
pub struct TokenBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub orientation: Orientation,
}

/// Convert a token's normalizedVertices to PDF points.
///
/// Normalized vertices use a top-left origin with y increasing downward.
/// PDF uses a bottom-left origin with y increasing upward.
pub fn compute_token_box(
    vertices: &[NormalizedVertex],
    orientation: Orientation,
    page_width_pts: f64,
    page_height_pts: f64,
) -> anyhow::Result<TokenBox> {
    if vertices.is_empty() {
        return Err(anyhow!("token has no normalizedVertices"));
    }

    let x_min = vertices.iter().map(|v| v.x).fold(f64::INFINITY, f64::min);
    let x_max = vertices.iter().map(|v| v.x).fold(f64::NEG_INFINITY, f64::max);
    let y_min = vertices.iter().map(|v| v.y).fold(f64::INFINITY, f64::min);
    let y_max = vertices.iter().map(|v| v.y).fold(f64::NEG_INFINITY, f64::max);

    // y_max in image coords (bottom of glyph) maps to the lowest PDF y value.
    let pdf_x = x_min * page_width_pts;
    let pdf_y = (1.0 - y_max) * page_height_pts;
    let pdf_width = (x_max - x_min) * page_width_pts;
    let pdf_height = (y_max - y_min) * page_height_pts;

    Ok(TokenBox {
        x: pdf_x,
        y: pdf_y,
        width: pdf_width,
        height: pdf_height,
        orientation,
    })
}
