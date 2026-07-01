use serde::de;
use serde::Deserialize;

fn parse_str_u64<'de, D>(d: D) -> Result<u64, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    s.parse::<u64>().map_err(de::Error::custom)
}

#[derive(Debug, Deserialize)]
pub struct Document {
    pub text: String,
    #[serde(rename = "shardInfo", default)]
    pub shard_info: ShardInfo,
    pub pages: Vec<Page>,
}

/// Build a lookup table mapping char index → byte offset in `text`.
/// The table has `text.chars().count() + 1` entries; the last entry is `text.len()`.
pub fn char_byte_offsets(text: &str) -> Vec<usize> {
    let mut v: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    v.push(text.len());
    v
}

/// Extract the substring `text[char_start..char_end]` using the precomputed offset table.
pub fn slice_chars<'a>(text: &'a str, offsets: &[usize], start: usize, end: usize) -> &'a str {
    let byte_start = offsets.get(start).copied().unwrap_or(text.len());
    let byte_end = offsets.get(end).copied().unwrap_or(text.len());
    &text[byte_start..byte_end]
}

#[derive(Debug, Deserialize, Default)]
pub struct ShardInfo {
    /// Absent on shard 0 → defaults to 0.
    #[serde(rename = "shardIndex", default, deserialize_with = "parse_str_u64")]
    pub shard_index: u64,
    #[serde(rename = "shardCount", default, deserialize_with = "parse_str_u64")]
    pub shard_count: u64,
    /// Global char offset where this shard's `text` starts in the full document.
    #[serde(rename = "textOffset", default, deserialize_with = "parse_str_u64")]
    pub text_offset: u64,
}

#[derive(Debug, Deserialize)]
pub struct Page {
    #[serde(rename = "pageNumber")]
    pub page_number: u32,
    pub dimension: Dimension,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Deserialize)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct Token {
    pub layout: Layout,
}

#[derive(Debug, Deserialize)]
pub struct Layout {
    #[serde(rename = "boundingPoly")]
    pub bounding_poly: BoundingPoly,
    #[serde(rename = "textAnchor")]
    pub text_anchor: TextAnchor,
    pub orientation: Option<Orientation>,
}

#[derive(Debug, Deserialize)]
pub struct BoundingPoly {
    #[serde(rename = "normalizedVertices")]
    pub normalized_vertices: Vec<NormalizedVertex>,
}

#[derive(Debug, Deserialize)]
pub struct NormalizedVertex {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct TextAnchor {
    #[serde(rename = "textSegments", default)]
    pub text_segments: Vec<TextSegment>,
}

#[derive(Debug, Deserialize)]
pub struct TextSegment {
    /// Absent when 0.
    #[serde(rename = "startIndex", default, deserialize_with = "parse_str_u64")]
    pub start_index: u64,
    #[serde(rename = "endIndex", deserialize_with = "parse_str_u64")]
    pub end_index: u64,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum Orientation {
    #[default]
    #[serde(rename = "PAGE_UP")]
    PageUp,
    #[serde(rename = "PAGE_RIGHT")]
    PageRight,
    #[serde(rename = "PAGE_DOWN")]
    PageDown,
    #[serde(rename = "PAGE_LEFT")]
    PageLeft,
}
