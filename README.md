# ocrtool

Creates searchable "sandwich" PDFs from [Google Cloud Document AI](https://cloud.google.com/document-ai) OCR output.

A sandwich PDF keeps the original scanned page image intact and adds an invisible text layer on top, so the file looks the same but text can be selected, copied, and searched in any PDF viewer.

## How it works

1. Google Document AI OCR runs on a scanned PDF and produces JSON output containing the extracted text and per-token bounding boxes (`normalizedVertices` in 0–1 page coordinates).
2. `ocrtool` reads the source PDF and the JSON shards, converts each token's bounding box from normalized image coordinates (top-left origin, y-down) to PDF points (bottom-left origin, y-up), and appends an invisible text content stream (`3 Tr` render mode) to each page.
3. The output PDF is visually identical to the input but fully searchable.

## Usage

```
ocrtool --input <PDF> --json <shard> [--json <shard> ...] --output <PDF> [--page <N>]
```

| Flag | Description |
|---|---|
| `--input` | Source scanned PDF |
| `--json` | Document AI JSON shard file; repeat for multi-shard documents |
| `--output` | Output PDF path |
| `--page` | Process only this page number (1-indexed); omit to process all pages |

### Single page (proof of concept)

```sh
ocrtool --input scan.pdf --json shard-0.json --output out.pdf --page 1
```

### All pages, multiple shards

Pass shards in ascending shard-index order:

```sh
ocrtool --input scan.pdf \
  $(printf ' --json shard-%d.json' $(seq 0 62)) \
  --output out.pdf
```

## Document AI JSON format

The tool expects the JSON produced by Document AI's **Document OCR** processor. Key fields used:

| Field | Description |
|---|---|
| `text` | Full extracted text for the shard |
| `shardInfo.textOffset` | Character offset of this shard's `text` within the full document |
| `pages[].pageNumber` | 1-indexed absolute page number (matched against PDF pages) |
| `pages[].tokens[].layout.boundingPoly.normalizedVertices` | Four vertices, coordinates in \[0, 1\] |
| `pages[].tokens[].layout.textAnchor.textSegments[].startIndex/endIndex` | Unicode codepoint offsets into the shard's `text` |
| `pages[].tokens[].layout.orientation` | `PAGE_UP` tokens are overlaid; rotated tokens are currently skipped |

`startIndex` is omitted when 0. All integer fields that arrive as JSON strings (int64) are handled automatically.

## Building

Requires Rust with Cargo. The crate pins `lopdf = "=0.29.0"` to stay compatible with Cargo < 1.76.

```sh
cargo build --release
```

The binary is written to `target/release/ocrtool`.

## Performance

On a 625-page, 63-shard document (~700 MB of JSON input):

- ~1.5 seconds end-to-end (release build)
- ~221 000 tokens overlaid

Each shard is parsed and dropped before the next is read, so peak heap usage is roughly one shard (~11 MB) plus the PDF in memory.

## Known limitations

- **Rotated tokens** (`PAGE_LEFT`, `PAGE_RIGHT`, `PAGE_DOWN`) are logged and skipped. These are rare in practice (vertical margin labels, footnote markers).
- **No horizontal scaling** (`Tz`): the invisible text is positioned and sized to match the token's bounding box height but not stretched to match its width. Text selection works correctly; exact cursor-position alignment is approximate.
- **WinAnsiEncoding** (Helvetica): covers Latin-1, including Finnish/Swedish characters (ä, ö, å). Characters outside Latin-1 are replaced with `?`.
- Shards must be passed in ascending `shardIndex` order. The tool processes them in the order given on the command line.

## License

MIT
