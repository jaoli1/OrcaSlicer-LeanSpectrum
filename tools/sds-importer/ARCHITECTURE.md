# SDS / TDS Importer — Architecture

## Pipeline overview

Two entry points: a single-PDF import path, and a vendor-catalog
crawl-then-batch-import path.

### Single PDF
```
  PDF (SDS or TDS)
        │
        ▼
  pdf::extract_text  ──► text non-empty? ──► yes ──┐
        │                                          │
        ▼ no                                       │
  ocr::run (tesseract)                              │
        │                                          │
        ▼                                          ▼
  ┌─────────────────────────────────────────────────┐
  │           sds::parse  /  tds::parse              │
  │                                                  │
  │  • ISO 11014-1 section split                     │
  │  • polymer::detect (CAS, names, abbreviations)   │
  │  • numeric extraction (T_g, T_melt, T_decomp,    │
  │    density, T_nozzle_recommended for TDS only)   │
  │  • language detection (fr / en at minimum)       │
  └─────────────────────────────────────────────────┘
        │
        ▼
  ExtractedFilament struct
        │
        ▼  if SDS only and manufacturer URL present
  fetcher::try_fetch_tds  ──► merge into ExtractedFilament
        │
        ▼
  profile::build_filament_profile
        │
        ▼
  Snapmaker_Orca user/<id>/filament/<name>.json
        │
        ▼
  profile::recommend_process_profile
        │
        ▼
  UI displays: "Created PLA Brand XYZ — use 0.20mm Standard process"
```

### Vendor catalog
```
  Vendor URL (e.g. https://eryone3d.com/pages/certificates)
        │
        ▼
  crawler::crawl_vendor_page
        │
        ▼
  ┌────────────────────────────────────────────────┐
  │  CrawlResult { entries: Vec<CatalogEntry> }    │
  │   • dedup by URL minus query string             │
  │   • classify_doc_type (SDS / TDS / Cert / ?)    │
  │   • classify_polymer  (PLA / PETG / …)          │
  │   • guess_product (anchor minus SDS/TDS tokens) │
  │   • skip non-filament certificates              │
  └────────────────────────────────────────────────┘
        │
        ▼  user picks N entries (auto-checked when
        │  doc_type ≠ Unknown)
        ▼
  import_from_urls (batched download_to_temp + import_pdf)
        │
        ▼
  BatchImportResult { succeeded, failed }
```

## Key invariants

1. **Never overwrite an existing user profile.** Save as
   `<name>.json` only if absent; otherwise append a `(1)`, `(2)`, …
   suffix.
2. **Always cross-check decomposition T.** The recommended nozzle temp
   in the generated profile must be at least 20 °C below the
   manufacturer-stated decomposition temperature.
3. **Refuse to invent missing data.** If a critical field (polymer
   family, nozzle temp range) is not present in either the PDF or the
   web TDS, the app emits a profile flagged `needs_review = true`
   and the user is asked to fill the gap in the UI before saving.
4. **OCR is the last resort.** Direct text extraction is tried first;
   OCR runs only if the text layer is empty or has less than 200
   characters total (likely a scanned PDF).

## SDS section mapping

ISO 11014-1 (and the EU REACH / GHS adaptations) defines 16 sections.
Mapping to our needs:

| Section | What we extract                                  |
|---------|--------------------------------------------------|
| 1       | Product name, manufacturer, contact URL          |
| 3       | Composition — polymer family by CAS or name      |
| 9       | Density, T_g, T_melt range, decomposition T      |
| 11      | Thermal decomposition products (safety warning)  |
| 16      | Revision date — used as profile "as_of" tag      |

Polymer detection uses a curated table of CAS numbers and IUPAC/common
names (see `data/polymer_signatures.json`):

| Polymer | Common identifiers                              |
|---------|-------------------------------------------------|
| PLA     | CAS 9051-89-2; "polylactic acid", "polylactide" |
| PETG    | CAS 25640-14-6 (PCTG variant); "PET-G", "PETG"  |
| ABS     | CAS 9003-56-9; "acrylonitrile butadiene styrene"|
| PC      | CAS 24936-68-3; "polycarbonate"                 |
| Nylon   | CAS 32131-17-2 (PA6), 32954-72-4 (PA12)         |
| TPU     | CAS 75880-72-1 family                           |
| ASA     | CAS 26299-47-8; "acrylonitrile styrene acrylate"|
| HIPS    | CAS 9003-55-8 + variants                        |

Detection is case-insensitive, matches across line breaks, and a single
match is enough — additives are ignored.

## TDS heuristics

TDS layouts are not standardised. Common labels (case-insensitive):

| Field           | Patterns matched                                          |
|-----------------|-----------------------------------------------------------|
| Nozzle temp     | "print(ing)? temp", "extruder temp", "nozzle"; followed by a range like `190-220 °C` |
| Bed temp        | "bed", "plate(form)?"; range like `40-60 °C`              |
| Print speed     | "speed", "vitesse"; range like `40-80 mm/s`               |
| Cooling fan     | "fan", "ventilateur"; "yes/no" or percentage              |
| Dry conditions  | "dry"/"sec"; hours + temperature                          |
| Flow rate       | "flow", "débit"; `mm³/s`                                  |

When SDS provides a melting range but no nozzle temp, we estimate:

```
T_nozzle_min = T_melt_max + 10        // clamped to polymer family minimum
T_nozzle_max = min(T_nozzle_min + 30, T_decomposition - 20)
T_recommended = (T_nozzle_min + T_nozzle_max) / 2
```

The recommended value is conservative and marked as estimated in the
profile metadata.

## Manufacturer URL fetching

When the SDS contains a URL such as `www.example.com/products/foo` and
the user toggles "fetch additional data online", the fetcher does:

1. Fetch the page with a 5 s timeout, single-page only (no crawling).
2. Look for direct links to `*.pdf` whose anchor text matches
   `TDS|Technical|Fiche technique|Datasheet`.
3. Download the first matching PDF (max 5 MB).
4. Re-run the SDS/TDS pipeline on the new file and merge fields
   into the existing `ExtractedFilament`.

If no link is found, the step is a no-op and the user is told.

## Profile output

A filament profile is a JSON object the slicer's user-profile loader
already understands. The schema is open (see `resources/profiles/`
in the upstream slicer). The minimum fields we write:

```json
{
  "name":                "Generic PLA — Brand X",
  "from":                "User",
  "type":                "filament",
  "inherits":            "Snapmaker PLA SnapSpeed @U1",
  "filament_type":       ["PLA"],
  "filament_vendor":     ["Brand X"],
  "nozzle_temperature":  ["205"],
  "nozzle_temperature_range_low":  ["190"],
  "nozzle_temperature_range_high": ["220"],
  "filament_density":    ["1.24"],
  "filament_flow_ratio": ["1"],
  "filament_cost":       ["20"],
  "_leanspectrum_metadata": {
    "source":      "SDS",
    "sds_path":    "<original-pdf-name>",
    "extracted_at":"<iso-date>",
    "needs_review": false,
    "estimated_fields": ["nozzle_temperature_range_low"]
  }
}
```

The `_leanspectrum_metadata` block is non-standard but ignored by the
slicer's loader. It lets the importer mark fields it had to estimate so
the user can review.

`inherits` points to the closest stock profile we know about. Inheriting
brings in retraction, pressure advance, fan, etc. for free.

## Process profile recommendation

The slicer ships dozens of process profiles per printer (different
nozzle sizes, layer heights, speed presets). After creating the filament
profile, the importer scans
`Snapmaker_Orca/system/<printer_vendor>/process/` and ranks the
profiles for the current printer + nozzle size:

```
score(p) = w_speed   * normalised_print_speed(p)        +
           w_quality * normalised_inverse_layer_h(p)    +
           w_match   * material_compat_bool(p, filament)
```

Weights come from a three-button UI: **Speed / Balanced / Quality**.
Default is Balanced.

The top-ranked profile is shown in the UI; the user can preview the
process settings before confirming.

## Cross-platform paths

```
macOS    ~/Library/Application Support/Snapmaker_Orca/user/<id>/filament/
Windows  %APPDATA%\Snapmaker_Orca\user\<id>\filament\
Linux    ~/.config/Snapmaker_Orca/user/<id>/filament/
```

`<id>` is the user UUID the slicer creates on first launch. We pick the
most recently modified directory matching the UUID pattern — if there
are several, we ask.

## Internationalisation

The parser handles French and English out of the box. The Tauri UI is
prepared for i18n via a `messages_<lang>.json` map; the same key set as
the slicer uses, so reusing translations later is trivial.

## Testing strategy

Test SDS / TDS PDFs live in `data/test_sheets/` (not committed —
copyright). The Rust test suite runs against a synthetic minimal PDF
generated from a markdown fixture, so the unit tests do not depend on
proprietary documents.

## Open questions for follow-up

- Should the importer write process profiles too (in addition to
  filament profiles)? Probably yes once we have validated recipes.
- Should we publish the polymer signature table as a community
  contribution path (filament makers submit PRs to add their products)?
  Yes — open this as Issue #2 once v0.1 ships.
- Mac App Store / Microsoft Store distribution — likely v1.0+, requires
  notarisation / code-signing certificates we don't have yet.
