# LeanSpectrum SDS / TDS Importer

> Companion desktop app that turns a filament manufacturer's Safety Data
> Sheet (SDS) or Technical Data Sheet (TDS) PDF into a ready-to-use
> Snapmaker_Orca / LeanSpectrum filament profile, and recommends the best
> matching process profile already shipped with the slicer.

## What it does

1. **Drop a PDF** (SDS or TDS, single file).
2. The app extracts text directly when the PDF is "born digital", and
   falls back to **Tesseract OCR** for scanned/image-only documents.
3. It parses the standard ISO 11014-1 / GHS section structure to extract:
   - Product identifier and manufacturer (section 1)
   - Polymer family (section 3 — composition / ingredients)
   - Density, glass transition T_g, melting range, decomposition T
     (section 9 — physical & chemical properties)
4. If only the SDS was provided and a manufacturer URL is present, the
   app optionally fetches the matching TDS for the printing temperatures
   the SDS does not carry.
5. From all the gathered data it builds a `.json` filament profile in the
   OrcaSlicer schema and writes it to the OS-specific Snapmaker_Orca
   profile directory:
   - macOS: `~/Library/Application Support/Snapmaker_Orca/user/<id>/filament/`
   - Windows: `%APPDATA%\Snapmaker_Orca\user\<id>\filament\`
   - Linux: `~/.config/Snapmaker_Orca/user/<id>/filament/`
6. It scans the installed process profiles for that nozzle size and
   recommends the most optimised one (speed, quality, or balanced — user
   picks the priority).

## Status

This is the **scaffold** of the app. The Rust crate compiles in
isolation, the UI shell renders, but the parsing logic only covers the
happy path on French + English SDS for the most common 5 polymer
families (PLA, PETG, ABS, TPU, Nylon). Iteration on real-world PDFs
will broaden coverage.

## Why a separate app?

The slicer is a 500k-LOC C++ codebase with its own build constraints.
Embedding PDF parsing + Tesseract OCR + an LLM-grade extractor inside it
would bloat every build and complicate cross-platform packaging. A
companion Tauri app:

- ships its own ~10 MB binary per OS (vs. Electron's ~80 MB)
- has its own release cycle decoupled from the slicer
- writes profiles to the slicer's existing user-profile directory, so
  no slicer modification is needed for import
- can be replaced or removed without touching the slicer

## Building

You need:

- Rust 1.75+ (`rustup default stable`)
- Node 18+ and `pnpm` (any package manager works, examples use pnpm)
- Tesseract 5+ installed system-wide (the app calls the binary; we do
  not bundle it to respect each distro's preferred install)
  - macOS: `brew install tesseract tesseract-lang`
  - Windows: `winget install UB-Mannheim.TesseractOCR`
  - Linux: `apt install tesseract-ocr tesseract-ocr-fra tesseract-ocr-eng`

Then from this directory:

```bash
pnpm install
pnpm tauri dev      # development run
pnpm tauri build    # release bundle
```

## Project layout

```
tools/sds-importer/
├── README.md                   # this file
├── ARCHITECTURE.md             # design notes (see below)
├── package.json                # Tauri / frontend tooling
├── frontend/                   # static HTML+JS UI (vanilla, no framework)
│   ├── index.html
│   └── src/main.js
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs             # Tauri command surface
│       ├── pdf.rs              # text extraction (born-digital path)
│       ├── ocr.rs              # Tesseract fallback
│       ├── sds.rs              # ISO 11014-1 section parser
│       ├── tds.rs              # TDS heuristics
│       ├── fetcher.rs          # optional manufacturer URL fetch
│       ├── polymer.rs          # polymer-family detection
│       └── profile.rs          # OrcaSlicer profile writer
└── data/
    ├── polymer_signatures.json # detection patterns (CAS, name regex)
    └── base_profiles/          # base process / filament profile stubs
```

## License & attribution

This subdirectory is licensed under AGPL-3.0, identical to the parent
slicer. It uses:

- [Tauri 2](https://tauri.app/) — Apache-2.0 / MIT
- [pdfium-render](https://crates.io/crates/pdfium-render) — Apache-2.0 / MIT
- [tesseract](https://github.com/tesseract-ocr/tesseract) — Apache-2.0
  (called as external binary; not statically linked)
- ISO 11014-1 / GHS standards are public references; no proprietary
  schema or profile content is reproduced.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the detailed design and
[doc/filament-economy/](../../doc/filament-economy/) for the parent
project's design notes.
