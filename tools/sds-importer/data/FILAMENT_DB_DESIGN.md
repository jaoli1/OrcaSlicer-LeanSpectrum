# Filament Database — Design Note

Pre-extracted filament database backing **MD Optimisateur**. This note covers
(a) the server-API + embedded-snapshot architecture, (b) the enrichment plan
for vendor document links and hex colors *without rehosting PDFs*, and (c) how
the database ties into the existing PDF parameter-extractor as a fallback.

The builder is `scripts/build_filament_db.py`; it produces
`tools/sds-importer/data/filaments.sqlite` and a `filaments.json` mirror.

---

## 0. Provenance & legal posture

The seed data is the open filament catalog published as JSON by the TigerTag
project (`TigerTag-Project/TigerTag-RFID-Guide`, `database/` folder). We honor
the following constraints, by construction:

| Constraint | How it is enforced |
|---|---|
| TigerTag **code/SDK is GPLv3** — do not embed/copy | We consume only the published JSON *data*. No upstream source is vendored. The builder is original code using the Python stdlib only. |
| Raw **facts are not copyrightable** | We store facts: brand names, recommended temps, densities, ids. No prose, no images, no schema lifted verbatim. |
| Do **not** name anything "TigerTag" in the product | The upstream id is kept only as internal cross-reference columns `tigertag_brand_id` / `tigertag_material_id`. No user-facing string says "TigerTag". |
| Do **not** store/redistribute vendor TDS/MSDS/RoHS PDFs | `document_refs` stores extracted **factual parameters + a SOURCE URL** that deep-links to the vendor's own hosted PDF. The PDF bytes are never copied into our DB or app bundle. |

A periodic refresh re-fetches the public JSON; the build is reproducible from
an offline snapshot via `--offline DIR` for CI.

### Schema (as built)

```
brands(id, name, website, tigertag_brand_id)
materials(id, brand_id, label, base_type, filled_type, density, diameter,
          tigertag_material_id, bambu_id, creality_id)
color_variants(id, material_id, color_name, hex, finish)
printing_params(id, material_id, nozzle_min, nozzle_max, bed_min, bed_max,
                dry_temp, dry_time, source)
document_refs(id, material_id, doc_type, url, retrieved_at,
              extraction_confidence, rohs_compliant)
```

`printing_params.source` is `'tigertag'` for seed rows; later sources use
`'tds'` (extracted from a vendor sheet), `'vendor'` (hand-curated), or
`'user'` (supplied via the importer fallback). The TigerTag mapping is
`nozzleTempMin/Max → nozzle_min/max`, `bedTempMin/Max → bed_min/max`,
`dryTemp/dryTime` direct, and `bambuID/crealityID` carried onto the material.
`base_type` (PLA/PETG/ABS/ASA/TPU/PC/PA…) and `filled_type` (CF/GF/…/NULL)
are derived from the label/fields. `color_variants` and `document_refs` are
created empty — colors live per physical NFC tag upstream (not in the
catalog), and document links are a later enrichment step.

---

## (a) Architecture: read-only server API + embedded snapshot

The database is **read-mostly**: the app never writes to the shared copy. Two
tiers serve it.

```
                 build_filament_db.py (periodic, CI)
                         │  fetch public JSON facts + enrichment
                         ▼
        ┌──────────────────────────────────┐
        │  canonical filaments.sqlite       │  (server side, versioned)
        │  + filaments.json mirror          │
        └──────────────┬───────────────────┘
                       │  read-only HTTP API
              GET /api/v1/filaments/version      → {schema_version, data_version, sha256}
              GET /api/v1/filaments/snapshot      → filaments.json (gzip, ETag)
              GET /api/v1/filaments/since/{ver}   → delta (optional optimisation)
                       │
                       ▼
        ┌──────────────────────────────────┐
        │  MD Optimisateur app              │
        │  ships an EMBEDDED snapshot        │  (works fully offline)
        │  refreshes opportunistically       │
        └──────────────────────────────────┘
```

**Server API.** A tiny stateless read service (no auth, cacheable behind a
CDN). It only ever serves the snapshot/version; all writes happen in the
build pipeline. Because the payload is static per data-version, it is trivially
cacheable and survives high request volume. The `.json` mirror — not the
sqlite binary — is the wire format (portable, diffable, gzip-friendly); the app
can rebuild a local sqlite from it or query the JSON directly.

**Embedded snapshot.** The app bundle ships the latest `filaments.json` at
release time, so a fresh install is fully functional offline. The bundled copy
carries its `data_version`.

**Versioning & refresh.**
- `schema_version` (integer in the mirror) gates structural changes; the app
  refuses a snapshot whose `schema_version` it doesn't understand and keeps the
  embedded one.
- `data_version` (monotonic, e.g. build timestamp or content hash) identifies
  the content. The app stores the active `data_version` in app-data.
- On launch (and at most once per N hours) the app calls `…/version`. If the
  server `data_version` is newer **and** `schema_version` is compatible, it
  fetches `…/snapshot`, validates `sha256`, and atomically swaps the local
  copy. On any failure it silently keeps the current copy.
- The embedded snapshot is the floor: the app never ends up with *less* data
  than it shipped with.

This keeps the hot path local (zero network for a query), updates without an
app re-release, and the legal posture intact (only factual JSON crosses the
wire).

---

## (b) Enrichment: vendor TDS/MSDS/RoHS **links** + hex colors, no rehosting

Two enrichment targets fill the currently-empty tables. Both are **additive
passes** over the built DB (a separate `enrich_*` script per concern), never
touching the seed import.

### B.1 Document links (`document_refs`) — link, never host

We store a **deep link to the vendor's own hosted PDF** plus the *facts* we
extracted from it — never the PDF bytes. Per-vendor discovery strategy, in
increasing cost order:

1. **Curated vendor map.** A small hand-maintained `vendors.json`
   (`brand → docs base URL / sitemap / product-page pattern`). Highest
   precision; covers the long-tail brands the catalog already names
   (Polymaker, eSUN, SUNLU, Prusament, Atome3D/ROSA3D, Bambu, …).
2. **Reuse the existing crawler.** `src-tauri/src/crawler.rs` already turns a
   single vendor catalog/product page into a classified list of PDF links
   (`DocType::{Sds,Tds,Certificate}` + a guessed polymer/product). The
   enrichment pass feeds each vendor's product/spec page through that same
   logic and keeps the links — we persist the **URL**, not the file.
3. **Match link → material.** Use the crawler's `guessed_polymer` /
   `guessed_product` against our `base_type`/`label` (and brand) to attach the
   link to the right `materials.id`. Ambiguous matches are written with a low
   `extraction_confidence` for human review rather than dropped.
4. **Record provenance.** Each row gets `doc_type` (`'TDS'|'MSDS'|'RoHS'`),
   `url` (the vendor deep link), `retrieved_at` (UTC), `extraction_confidence`
   (0–1, from the match quality), and `rohs_compliant` (`1/0/NULL`) parsed from
   a RoHS declaration's text when one is found.
5. **Optionally extract facts at discovery time.** When a TDS link is found we
   may run the *existing* parser (see (c)) once to backfill
   `printing_params(source='tds')` / `density`, then discard the downloaded
   bytes. Only the extracted numbers + the URL persist.

A periodic re-validation re-checks each stored `url` (HEAD request) and flags
dead links by lowering confidence, so we never serve a 404 to users.

### B.2 Hex colors (`color_variants`)

The TigerTag *catalog* has no colors (they live per physical NFC tag), so this
table starts empty and is filled from vendor-facing sources:

1. **Vendor color lists.** The same curated vendor map points at each brand's
   color page / product JSON. Scrape `color_name` + swatch `hex` + `finish`
   (matte/silk/glow/transparent/…) and attach to the matching `materials.id`.
2. **Public swatch APIs.** Where a vendor exposes a structured palette (e.g. a
   storefront product API) we ingest `name`/`hex`/`finish` directly — these are
   short factual values, same legal footing as the temps.
3. **Normalise.** Store `hex` as `#RRGGBB` uppercase; map finish synonyms to a
   small controlled vocabulary; dedupe per material.
4. **Confidence & review.** Colors whose hex was approximated (named color with
   no swatch) are flagged for review rather than presented as exact.

Both passes are idempotent (`INSERT OR REPLACE` keyed on natural keys) so they
can run on a schedule alongside the catalog refresh.

---

## (c) Tie-in with the existing PDF parameter-extractor (fallback path)

The importer crate (`tools/sds-importer/src-tauri/`) already parses vendor
sheets into an `ExtractedFilament` (`lib.rs`) via two heuristic parsers:

- **`sds.rs`** — splits a Safety Data Sheet into the ISO 11014-1 / GHS
  16-section structure and pulls density, melt/decomposition/glass-transition
  temps, manufacturer, URL, and (for modern SDS that embed them) print/bed
  temps from Section 9.
- **`tds.rs`** — vendor-specific Technical Data Sheet heuristics: label-keyword
  matching + windowed numeric-range extraction for nozzle/bed/print-speed,
  density, T_g (Vicat proxy), plus the authoritative "test-specimen printed
  under the following conditions" override.

**"0 sheets found → user supplies a TDS" fallback.** This is the bridge
between the pre-extracted DB and the existing extractor:

1. A user picks a filament. We look it up in the embedded snapshot
   (brand + `base_type`/`filled_type` → `materials` → `printing_params`).
2. **Hit:** serve the pre-extracted params immediately (zero network). If a
   `document_refs` TDS link exists, surface it as "vendor spec sheet" — a link
   out to the vendor PDF, still not rehosted.
3. **Miss / sparse** (no params, or the brand isn't in the catalog — `brand_id`
   is NULL on seed rows until enrichment, so brand-specific lookups commonly
   fall here): fall back to the existing importer. The user supplies the vendor
   TDS/SDS (file or URL); `tds.rs`/`sds.rs` produce an `ExtractedFilament`.
4. **Promote the result.** The extracted values are written back as a new
   `materials` row (or attached to the matched one) with
   `printing_params(source='tds')`, and — if the user provided a URL — a
   `document_refs` row pointing at *their* source link (never the bytes). The
   contribution now benefits the next lookup.

Field mapping `ExtractedFilament → DB`:

| ExtractedFilament | DB column |
|---|---|
| `polymer.as_str()` | `materials.base_type` |
| `density_g_cm3` | `materials.density` |
| `nozzle_temp_min_c` / `max_c` | `printing_params.nozzle_min` / `max` |
| `bed_temp_min_c` / `max_c` | `printing_params.bed_min` / `max` |
| (`dryTemp`/`dryTime` not in TDS) | left from seed, or NULL |
| `manufacturer_url` | `document_refs.url` (`doc_type` per source) |
| `needs_review` / estimated fields | low `extraction_confidence` |

The DB makes the *common* case instant and offline; the existing extractor
remains the escape hatch for anything the catalog doesn't cover, and every
manual import enriches the shared dataset.

---

## (d) Manufacturer enrichment + source precedence

> **Authoritative source = the manufacturer's own website.** The TigerTag
> seed is a *convenience index only*; its parameters, sheets and colors are
> **low confidence** and are superseded by manufacturer data wherever the two
> disagree. Implemented by `scripts/scrape_manufacturer_data.py` (additive).

### Precedence rule

`printing_params.source` and `document_refs` now carry an explicit provenance.
Resolution is **highest-precedence row wins, per (material, field)**:

| Precedence | `source` value | Meaning | Confidence |
|---:|---|---|---|
| 1 (highest) | `manufacturer` | Read from the vendor's own TDS/SDS/color page | authoritative |
| 2 | `tds` / `vendor` | Extracted by the PDF extractor from a vendor sheet, or hand-curated | high |
| 3 | `user` | Supplied via the importer fallback | medium |
| 4 (lowest) | `tigertag` | **Seed only** — treat as a hint, never override a higher tier | low / seed |

So a query layer should `ORDER BY` this precedence and take the first non-null
value for each field. Seed (`tigertag`) rows are kept for coverage of brands
not yet enriched, but are shadowed the moment a `manufacturer` row exists for
the same material. `document_refs.extraction_confidence` (0–1) further ranks
links within a tier (e.g. a TDS mirror gets 0.7; a vendor-hosted PDF gets 1.0),
and `rohs_compliant` is set to `1` only when a RoHS declaration / test report is
actually found (Prusament SDS, Eryone RoHS reports); a plain SDS with no RoHS
statement leaves it `NULL` (unknown) rather than guessing.

### Enrichment model (how manufacturer data attaches)

The seed `materials` rows are brand-agnostic *recipes* (`brand_id` NULL — colors
and brand live on the physical tag upstream, not in the catalog). Manufacturer
data is brand-*specific*, so the enricher does **not** retro-fit a seed recipe.
Instead it inserts a clearly-attributed brand-specific `materials` row (id in a
collision-free high band, `>= 1_000_000`) linked to the matched `brands.id`, and
hangs the `document_refs` / `color_variants` / `printing_params(source=
'manufacturer')` off it. Brand match is by name (case-insensitive, contains-
tolerant); `brands.website` is backfilled when NULL. All inserts are idempotent
(natural-key existence checks), so the pass is safe to run on a schedule next to
the catalog refresh. **Legal posture is unchanged**: only deep-links to the
vendor's own PDFs + extracted facts + hex are stored; no PDF is ever rehosted.

> Note: the `filaments.json` mirror is regenerated from the sqlite by
> `build_filament_db.py`; this enrichment pass writes the sqlite only, so the
> mirror is refreshed on the next build (kept out of this additive pass to
> avoid touching a build artifact).

### Adapter framework

`scrape_manufacturer_data.py` is a pluggable **adapter registry**:

* `ManufacturerAdapter` — base interface. `discover()` returns, per product
  line, a `MaterialRecord` (TDS / MSDS / RoHS urls + flag, hex colors, printed
  params). `brand_names` binds catalog spellings to the adapter.
* **Per-vendor adapters** encode the *verified* document URLs + color→hex
  tables for a brand (the pilots below).
* `GenericFallbackAdapter` — long-tail strategy used when no dedicated adapter
  is registered: site-scoped search (`site:<domain> "<phrase>" filetype:pdf`
  over *technical data sheet / TDS / safety data sheet / SDS / MSDS / RoHS /
  REACH*) to find deep-links, plus swatch-hex harvesting from product/color
  pages, emitted at a **lower** `extraction_confidence` (~0.4) for review.
* `--verify` HTTP-checks every link (expect 200/206 + `application/pdf`).

### Pilot results (5 brands, verified)

All 18 document links below returned **HTTP 200/206 + `application/pdf`** on
2026-05-28; params/hex were read from each vendor's own published sheet/page.

| Brand | Lines populated | Docs (type) | Hex source | RoHS |
|---|---|---|---|---|
| **Polymaker** | PolyTerra PLA, PolyLite PETG | TDS + SDS (polymaker.com) | product page swatches (exact) | NULL (SDS, no RoHS stmt) |
| **Prusament** | Prusament PLA | TDS + SDS + RoHS (prusament.com) | help.prusa3d.com hex table (exact) | **1** (SDS: no substances > 2011/65/EU) |
| **Bambu Lab** | PLA Basic | TDS + MSDS (wiki.bambulab.com) | official Hex Code PDF (exact) | NULL (MSDS, no RoHS stmt) |
| **SUNLU** | SUNLU PLA, SUNLU PETG | TDS + SDS (sunlu.com / mirror) | store color *names* only (hex NULL) | NULL |
| **Eryone** | Eryone PLA, Eryone PETG | RoHS + REACH + MSDS + TDS (eryone3d CDN) | store color *names* only (hex NULL) | **1** (dedicated RoHS test reports) |

### Scaling to all 122 brands — fan-out plan

The goal is to cover the remaining ~117 brands by **fanning out many agents
over the shared framework**: each agent owns a cluster of brands, writes (or
reuses) adapters against the *same* `ManufacturerAdapter` interface, and runs
`--verify` before committing rows. Clusters are sized by discovery difficulty:

**Tier A — clean, structured TDS/MSDS portals (per-vendor adapters, high ROI).**
Vendors with a predictable docs URL pattern or a documents/wiki hub. Examples
from the seed: Polymaker, Prusament, Bambu Lab, Eryone (done); plus eSUN,
Fillamentum, ColorFabb, FormFutura, 3DXTech, Add:North, Extrudr, Spectrum,
Fiberlogy, ngen/ColorFabb, Polymaker-OEM lines. One adapter each; colors via
product pages. Cluster ≈ 25–30 brands across ~3 agents.

**Tier B — Shopify / WooCommerce storefronts (semi-generic adapter).** Colors
are reliably exposed at `/<product>.json` (Shopify) or via product variation
JSON (Woo); docs live on the store CDN (`cdn.shopify.com/.../files/*.pdf`) and
are found by the generic search. SUNLU and Eryone are this shape. A *shared*
storefront adapter parametrised by domain handles most of these; per-brand work
is just the domain + product handles. Cluster ≈ 40–50 brands across ~4 agents.
Hex is often name-only here → store names with `hex=NULL`, confidence 0.5.

**Tier C — generic fallback only (no structured source).** Small/OEM/rebrand
brands with a thin site or docs behind JS/PDF-image scans. Use
`GenericFallbackAdapter`; expect partial coverage (often SDS but no TDS, or
named colors only), all at low confidence and flagged for human review. RoHS
frequently only as an image (`.jpg` cert) → record the link, leave the flag
NULL unless OCR confirms. Cluster ≈ 40–50 brands across ~3–4 agents.

**Shared mechanics for every agent**
1. Resolve the official domain (WebSearch), set it as the adapter `website`
   and backfill `brands.website`.
2. Discover TDS/MSDS/RoHS deep-links; classify by filename/anchor text; record
   `doc_type`, `url`, `retrieved_at`, `extraction_confidence`, `rohs_compliant`.
3. Read params from the TDS (nozzle/bed/density) → `printing_params(source=
   'manufacturer')`; read hex from swatches → `color_variants` (normalised
   `#RRGGBB`, finish vocabulary).
4. `--verify` all links; drop/flag any non-PDF or dead link.
5. Idempotent INSERTs only; never touch seed rows, app/Rust code, profiles or
   the build. The framework + precedence rule mean clusters merge cleanly.

**Known difficulty notes.** Bambu/Prusa/Polymaker publish exact hex (best
case). Chinese-market brands (SUNLU, Eryone, Kingroon, Geeetech, JAYO) tend to
publish SDS/RoHS but only color *names* → hex stays NULL. CF/GF/specialty lines
often lack a public TDS → params fall back to seed. A periodic re-validation
job re-checks stored URLs (the `--verify` path) and lowers confidence on dead
links so users are never served a 404.
