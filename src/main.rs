use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

mod model;
mod pdf;
mod transform;

struct Args {
    input: PathBuf,
    json_files: Vec<PathBuf>,
    output: PathBuf,
    page: Option<u32>,
}

fn parse_args() -> Result<Args> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut json_files: Vec<PathBuf> = Vec::new();
    let mut output: Option<PathBuf> = None;
    let mut page: Option<u32> = None;

    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--input" => {
                i += 1;
                input = Some(PathBuf::from(raw.get(i).context("--input requires a value")?));
            }
            "--json" => {
                i += 1;
                json_files.push(PathBuf::from(raw.get(i).context("--json requires a value")?));
            }
            "--output" => {
                i += 1;
                output = Some(PathBuf::from(raw.get(i).context("--output requires a value")?));
            }
            "--page" => {
                i += 1;
                let s = raw.get(i).context("--page requires a value")?;
                page = Some(s.parse().context("--page must be a positive integer")?);
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: ocrtool --input <PDF> --json <shard> [--json <shard>...] --output <PDF> [--page <N>]"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {}", other),
        }
        i += 1;
    }

    Ok(Args {
        input: input.context("--input is required")?,
        json_files,
        output: output.context("--output is required")?,
        page,
    })
}

fn main() -> Result<()> {
    let args = parse_args()?;

    if args.json_files.is_empty() {
        bail!("at least one --json shard file is required");
    }

    let mut doc = lopdf::Document::load(&args.input)
        .with_context(|| format!("loading {:?}", args.input))?;

    let page_map = doc.get_pages();

    // Pre-create one Helvetica font object shared across all pages.
    let font_id = {
        let mut d = lopdf::Dictionary::new();
        d.set(b"Type", lopdf::Object::Name(b"Font".to_vec()));
        d.set(b"Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        d.set(b"BaseFont", lopdf::Object::Name(b"Helvetica".to_vec()));
        d.set(b"Encoding", lopdf::Object::Name(b"WinAnsiEncoding".to_vec()));
        doc.add_object(lopdf::Object::Dictionary(d))
    };

    let mut total_tokens = 0usize;
    let mut total_pages = 0usize;

    for json_path in &args.json_files {
        let file = File::open(json_path)
            .with_context(|| format!("opening {:?}", json_path))?;
        let shard: model::Document = serde_json::from_reader(BufReader::new(file))
            .with_context(|| format!("parsing {:?}", json_path))?;

        // Build char-index → byte-offset table once per shard.
        let char_offsets = model::char_byte_offsets(&shard.text);

        for page in &shard.pages {
            if let Some(target) = args.page {
                if page.page_number != target {
                    continue;
                }
            }

            let page_id = match page_map.get(&page.page_number) {
                Some(&id) => id,
                None => {
                    eprintln!("warning: page {} not found in PDF (skipping)", page.page_number);
                    continue;
                }
            };

            let (page_width, page_height) = pdf::get_page_dimensions(&doc, page_id)
                .with_context(|| format!("getting dimensions for page {}", page.page_number))?;

            let mut token_boxes: Vec<(transform::TokenBox, String)> = Vec::new();

            for token in &page.tokens {
                let orientation = token.layout.orientation.unwrap_or_default();

                if orientation != model::Orientation::PageUp {
                    eprintln!("info: skipping {:?} token on page {}", orientation, page.page_number);
                    continue;
                }

                let vertices = &token.layout.bounding_poly.normalized_vertices;
                let tb = match transform::compute_token_box(vertices, orientation, page_width, page_height) {
                    Ok(tb) => tb,
                    Err(e) => {
                        eprintln!("warning: {e}");
                        continue;
                    }
                };

                // Extract text via codepoint indices.
                let mut text = String::new();
                for seg in &token.layout.text_anchor.text_segments {
                    text.push_str(model::slice_chars(
                        &shard.text,
                        &char_offsets,
                        seg.start_index as usize,
                        seg.end_index as usize,
                    ));
                }

                if text.trim().is_empty() {
                    continue;
                }

                token_boxes.push((tb, text));
            }

            let count = token_boxes.len();
            let content = pdf::build_text_stream(&token_boxes);
            pdf::add_text_overlay(&mut doc, page_id, font_id, content)
                .with_context(|| format!("adding overlay to page {}", page.page_number))?;

            eprintln!("page {}: {} tokens overlaid", page.page_number, count);
            total_tokens += count;
            total_pages += 1;
        }
    }

    doc.save(&args.output)
        .with_context(|| format!("saving {:?}", args.output))?;

    eprintln!(
        "done: {} pages processed, {} tokens total → {:?}",
        total_pages, total_tokens, args.output
    );
    Ok(())
}
