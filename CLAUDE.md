# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build

```sh
cargo build --release
# binary at target/release/ocrtool
```

Toolchain is Rust edition 2021. `lopdf` is pinned to `=0.29.0` — do not upgrade it. Versions 0.30+ pull in `time ^0.3` which requires Rust edition 2024 and breaks Cargo 1.75. Do not add `clap`; clap 4.5+ pulls in `clap_lex` with edition 2024.

## Run

```sh
./target/release/ocrtool \
  --input scan.pdf \
  $(printf ' --json shard-%d.json' $(seq 0 N)) \
  --output out.pdf
```

Shards must be passed in ascending `shardIndex` order. Sample data lives in `sample/` (gitignored).

## Architecture

The tool reads a scanned PDF and Google Document AI JSON shards, then appends an invisible text overlay (PDF render mode `3 Tr`) to each page, producing a searchable sandwich PDF.

**Pipeline** (all in `main.rs`):
1. Load source PDF with lopdf
2. Pre-create one shared Helvetica Type1 font object (WinAnsiEncoding) — reused on every page
3. For each JSON shard: parse → iterate pages → collect token bounding boxes → build text stream → append to page

**Module responsibilities:**

- `src/model.rs` — serde structs for Document AI JSON. Key detail: `shardInfo` integer fields (`shardIndex`, `shardCount`, `textOffset`) and `textAnchor.textSegments[].startIndex/endIndex` are int64 encoded as JSON strings. A custom `parse_str_u64` deserializer handles this. `startIndex` is absent when 0 (use `#[serde(default)]`). Text offsets are Unicode codepoint indices into the shard's own `text` field, not byte offsets — `char_byte_offsets()` builds a lookup table.

- `src/transform.rs` — converts `normalizedVertices` (0–1, top-left origin, y-down) to `TokenBox` in PDF points (bottom-left origin, y-up): `pdf_x = x_min * W`, `pdf_y = (1 - y_max) * H`. Uses min/max over all vertices for robustness.

- `src/pdf.rs` — all lopdf interaction:
  - `get_page_dimensions`: walks page tree to find inherited `MediaBox`
  - `build_text_stream`: emits the BT/ET content stream; `add_text_overlay`: appends stream to `/Contents` and injects font into `/Resources/Font` (handles both inline and indirect Resources dicts)

## Invisible text layer details

Font: Helvetica Type1, `Tf` size 1. The text matrix diagonal serves as the effective font size.

**Per-token PDF operators:**
```
<a> <b> <c> <d> <tx> <ty> Tm    % rotation + position
<tz_percent> Tz                  % horizontal scale to fill bbox width
<hex bytes> Tj                   % WinAnsiEncoding text
```

**Orientation → text matrix** (fs = font size, bbox origin at bottom-left `(x, y)`):

| Orientation | a | b | c | d | tx | ty |
|---|---|---|---|---|---|---|
| PAGE_UP | fs | 0 | 0 | fs | x | y |
| PAGE_RIGHT | 0 | −fs | fs | 0 | x | y+h |
| PAGE_LEFT | 0 | fs | −fs | 0 | x+w | y |
| PAGE_DOWN | −fs | 0 | 0 | −fs | x+w | y+h |

For PAGE_UP/PAGE_DOWN: `fs = bbox.height`, Tz fills `bbox.width`.
For PAGE_LEFT/PAGE_RIGHT: `fs = bbox.width`, Tz fills `bbox.height`.

**Tz formula:** `Tz = (desired_advance / natural_width) * 100`, clamped to [10, 1000].
`natural_width = sum_of_glyph_advances(encoded_bytes) * fs / 1000` using the Helvetica AFM table in `helvetica_width()`.

**Encoding:** `winansi_encode()` maps chars to WinAnsiEncoding bytes. Latin-1 (U+0020–U+00FF) maps directly; Windows-1252 extended chars (U+0100+, e.g. † → 0x86, – → 0x96, € → 0x80) are mapped explicitly. Unmapped chars become `?`.
