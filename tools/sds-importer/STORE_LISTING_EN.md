# Product Listing — Filament & Print-Profile Optimiser by Maison Drabiec

> Working document for the owner to review. **Do not publish as-is.**
> All figures are phrased as "up to" and reflect the software's real state (v0.1.19).

---

## 1. Product title

**Chosen title (< 70 characters)**

> **MD Optimiser — ready-to-print filament & process profiles**

**Alternative variants**

- *MD Filament Optimiser — reliable 3D settings, zero guesswork*
- *MD Optimiser: turn any spec sheet into a tuned print profile*
- *MD Optimiser for Snapmaker U1 — filament + process in one click*

---

## 2. Hook / subtitle

**Drop your filament's spec sheet, get a tuned print profile back. No more hours of trial-and-error.**

---

## 3. Short description (store thumbnail, ≈ 300 characters)

> Turn any manufacturer spec sheet (SDS/TDS PDF) or catalog URL into a tuned **filament** profile plus ready-to-print **process** profiles for Snapmaker_Orca. Backed by 700+ materials sourced from official manufacturer sites. Reliable nozzle, bed, drying and flow settings — no expertise required. Windows / macOS / Linux.

---

## 4. Long description

### Stop guessing your settings

Every new spool means the same chore: track down the spec sheet, decode a temperature table, run a calibration, ruin a first print… then start over. The **MD Optimiser** removes that step. Drop your filament's manufacturer sheet (the SDS or TDS PDF), or paste the URL of a brand's "certificates" page, and the software builds a ready-to-print profile **automatically**.

You no longer need to be an extrusion expert: nozzle temperature, bed temperature, drying, density and volumetric flow are populated from the **manufacturer's own official data** — not approximate values scraped from somewhere else.

### Trustworthy data, not forum copy-paste

The software's real strength is its **database of 700+ materials** (709 entries, 122 brands), built to prioritise **official manufacturer sites and sheets**. The internal rule is strict: whenever manufacturer data exists (Polymaker, Prusament, Bambu Lab, eSUN, SUNLU, Eryone…), it **always takes precedence** over any other source. For each material you get the recommended temperatures, density, drying conditions, colour codes, and **direct links to the official TDS / MSDS / RoHS sheets** hosted by the manufacturer itself.

> Important: the software **never** re-hosts manufacturer PDFs. It stores the useful facts (temperatures, density…) and a deep link to the original document.

### Extraction that actually reads the sheet

When you import a PDF, the parser doesn't stop at the parameter table. It also reads the **"test specimen" note** — the test conditions many manufacturers state ("all specimens are printed at 210 °C, 80 mm/s, bed 60 °C"). Those values are **authoritative** and override the table's midpoints, because they describe exactly how the manufacturer obtained its mechanical results. Built-in safety: the chosen nozzle temperature always stays below the stated decomposition threshold.

### A library of PROCESS profiles by project type

Beyond the filament profile, the MD Optimiser generates **process profiles by project type** for **any printer** supported by OrcaSlicer (Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…): you pick **brand → model → nozzle** and the app produces the 7 profiles tuned for that machine. A one-click button also generates the full **Snapmaker U1** set (7 types × 4 nozzles = 28 profiles):

- **Fast prototype** — thick layers, top speed, high accelerations
- **Everyday object** — the strength / speed / finish balance
- **Figurine** — fine layers, tight cornering and low acceleration/jerk to suppress vertical artefacts and resonance (VFA)
- **Vase** — spiral mode, single wall
- **Decoration** — surface ironing, refined finish
- **Toy** — reinforced walls, generous infill
- **Mechanical part** — multiple walls, dense infill

Each profile is tuned for **cornering** and **resonance / VFA** (via acceleration and jerk limits) and to stay under the U1's flow ceiling. Filament-specific tuning (temperatures, flow, retraction) stays on the filament profile: one shared set of processes plus per-material tuning is all it takes.

### Fork features enabled out of the box

The generated profiles activate the **Snapmaker_Orca / OptimusOrca** fork capabilities:

- **Filament economy** — shrinks wipe-tower purges (−30 % by default for a freshly-used nozzle), removes redundant tool changes, and scales extrusion by curvature. On multi-colour prints, savings can reach **up to −15 to −30 %** depending on the part and the number of colour changes.
- **Scarf seams** — a sloped joint that makes the Z-seam nearly invisible on most materials (disabled on TPU, which doesn't ramp cleanly).
- **Colour-mixing readiness** — the safe optimisation (region-collapse) is enabled; experimental modes (gradient, dithering…) stay off by default so single-colour prints are unaffected.

### Always current, lightweight, bilingual

The database receives **regular updates**: the app ships an offline-capable snapshot and refreshes automatically when a newer data version is available. The interface is **bilingual French / English**, the binary is **lightweight** (Tauri, ≈ 10 MB per OS) and runs on **Windows, macOS and Linux**.

---

## 5. Technical specifications

| Specification | Detail |
|---|---|
| Product type | Desktop application (utility for FDM 3D printing) |
| Operating systems | Windows · macOS (Apple Silicon & Intel) · Linux |
| Distribution format | **A single ZIP** with three folders: Windows (**`.exe`**), macOS (**`.dmg`**), Linux (**`.AppImage`**) |
| Target slicers | **OrcaSlicer family**: OrcaSlicer · Creality Print · Bambu Studio · SnapmakerOrca / OptimusOrca (profiles written to the slicer's user folder) |
| Accepted inputs | Manufacturer spec-sheet PDF (SDS / TDS), catalog/certificates page URL, local PDF folder |
| Generated outputs | Filament profile `.json` + process profiles `.json` (by project type, for the chosen printer) |
| Database | 700+ materials (709 entries, 122 brands), official manufacturer data prioritised |
| Printers covered | OrcaSlicer family — 57 brands / 326 models (Creality, Bambu, Snapmaker, Anycubic, Prusa…); all their nozzles |
| Interface languages | French · English (switchable, remembered) |
| Technology | Tauri (lightweight binary, ≈ 10 MB per OS) |
| OCR (scanned PDFs) | Supported via system-installed Tesseract (text PDFs work without it) |
| Internet connection | Optional (online TDS lookup and database updates); the core works offline |
| Software licence | AGPL-3.0-or-later |

> Requirements: a Windows 10/11 PC, recent macOS, or Linux machine capable of running the Snapmaker_Orca slicer. No heavy dependencies; OCR is only needed for scanned (image) sheets.

---

## 6. What's included

- The **MD Optimiser** application for your system (Windows / macOS / Linux).
- The **database of 700+ materials** (temperatures, density, drying, colours, links to official sheets).
- The **filament-profile generator** from an SDS/TDS PDF, a catalog URL, or a local folder.
- **Process-profile generation** by project type for any OrcaSlicer-family printer (+ the full Snapmaker U1 set).
- The **update checker** (auto database + new-version prompt) and material-adaptive **supports / anti-warp** settings.
- **Fork features enabled**: filament economy, scarf seams, colour-mixing readiness.
- **Batch import** from a manufacturer's "certificates" page.
- The **bilingual FR / EN** interface.
- **Regular database updates**.

---

## 7. SEO

**Keywords**

`Snapmaker U1 filament profile` · `3D printing optimiser` · `OrcaSlicer profiles` · `automatic filament settings` · `multicolor filament saving` · `import TDS SDS filament sheet` · `3D printing process profiles` · `Snapmaker_Orca`

**Meta description (≈ 155 characters)**

> Generate tuned filament and process profiles for Snapmaker_Orca from any manufacturer spec sheet. 700+ official materials. Windows/macOS/Linux.

---

## 8. FAQ

**Is it compatible with my printer / slicer?**
Yes, for the whole **OrcaSlicer family**: OrcaSlicer, Creality Print, Bambu Studio, SnapmakerOrca / OptimusOrca. The app covers **their printers** (57 brands, 326 models) — you pick brand, model and nozzle, and the profiles appear in the slicer's menus once generated. PrusaSlicer (`.ini` format) is planned next.

**Where does the data come from? Is it reliable?**
The database prioritises **official manufacturer sites and sheets**. Whenever manufacturer data exists, it always takes precedence over any other source. For each material you get **links to the official sheets** (TDS / MSDS / RoHS) hosted by the manufacturer. The software never re-hosts those PDFs.

**How is this better than a generic "PLA" profile?**
A generic profile applies averages. The MD Optimiser starts from **your specific filament's sheet**: it reads the parameter table *and* the "test specimen" note (the authoritative test conditions) to dial in nozzle, bed, speed and flow precisely, then adds process profiles designed for your project type.

**My PDF is a scan / an image — will it work?**
Yes, provided you install **Tesseract** on your system (OCR). Manufacturers' "text" PDFs — the vast majority — work with nothing extra installed.

**Are database updates included?**
Yes. The app ships an offline-capable snapshot and updates automatically when new data is published, with no need to reinstall the software.

**How much filament can I realistically save?**
On **multi-colour** prints, savings can reach **up to −15 to −30 %** depending on the part and the number of colour changes (purge shrinking, removal of needless tool changes). On single-colour prints the gain comes mostly from curvature-based scaling. These are ballpark figures, not a guaranteed number.

**Refund policy?**
*(To be completed by the owner per the store's terms — e.g. refund within X days if the software fails to launch on the stated configuration.)*

---

## 9. Visual / screenshot ideas to prepare

- **Hero screenshot**: the "Single PDF" window with a sheet dropped in and the generated profile shown (a "Ready" badge).
- **Before / after**: a raw manufacturer spec-sheet table, an arrow, then the finished filament profile in the slicer's menu.
- **"Process library" tab**: the grid of 7 project types × 4 nozzles (28 profiles) with the "Generate" button.
- **Thumbnails of the 7 project types**: one icon/render per intent (Prototype, Figurine, Vase, Decoration, Toy, Mechanical part, Everyday object).
- **"Filament economy" diagram**: purge tower before/after, captioned "up to −15 to −30 % on multi-colour".
- **"Scarf seam" close-up**: comparison of a classic Z-seam vs a scarf seam.
- **"Vendor catalog" tab**: a parsed certificates page, the list of detected PDFs with SDS/TDS badges and batch import.
- **OS logo strip**: Windows / macOS / Linux plus "bilingual FR/EN" and "lightweight (Tauri)".
- **"700+ materials" visual**: a brand cloud + key figures (709 entries, 122 brands) with the note "official manufacturer sources".
