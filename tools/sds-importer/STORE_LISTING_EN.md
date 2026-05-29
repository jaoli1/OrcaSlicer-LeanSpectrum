# Product Listing — Filament & Print-Profile Optimiser by Maison Drabiec

> Working document for the owner to review. **Do not publish as-is.**
> All figures are phrased as "up to" and reflect the software's real state (v0.4.0).

---

## 1. Product title

**Chosen title (< 70 characters)**

> **MD Optimiser — 700+ filaments, profile in one click**

**Alternative variants**

- *MD Optimiser — 700+ official filaments, profiles in one click*
- *MD Optimiser: pick your filament, get your profiles*
- *MD Optimiser for Snapmaker U1 — filament + process in one click*

---

## 2. Hook / subtitle

**Pick your printer, choose your filament from a database of 700+ official materials, and generate the filament profile plus its process profiles in one click. No more hours of trial-and-error.**

---

## 3. Short description (store thumbnail, ≈ 300 characters)

> Pick your printer, choose your filament from a **database of 700+ materials** built from manufacturers' own official sheets (TDS / SDS / MSDS / RoHS), and generate **in one click** a tuned **filament** profile plus its ready-to-print **process** profiles for Snapmaker_Orca. For a filament not yet in the database, importing a manufacturer PDF still works. Reliable nozzle, bed, drying and flow settings — no expertise required. Windows / macOS / Linux.

---

## 4. Long description

### Pick your filament, the profile is one click away

Every new spool means the same chore: track down the spec sheet, decode a temperature table, run a calibration, ruin a first print… then start over. The **MD Optimiser** removes that step. You **pick your slicer** (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, or a custom folder), **pick your printer** at the top of the window (brand → model → nozzle, or "all nozzles"), **search for your filament in the library** (700+ materials built from manufacturers' own official sheets — with a **brand** filter to go straight to it), and a **single click** generates the tuned **filament profile** *and* its **7 process profiles by project type** together. You can even **tick several materials** at once: the app then creates one filament profile per material, plus a single shared set of the 7 process profiles.

You no longer need to be an extrusion expert: nozzle temperature, bed temperature, drying, density and volumetric flow are populated from the **manufacturer's own official data** — not approximate values scraped from somewhere else. The generated profiles are written straight into the preset folder of the **slicer you chose**, and that choice is remembered between sessions.

### Trustworthy data, not forum copy-paste

The heart of the software is its **database of 700+ materials** (709 entries, 122 brands), built from **manufacturers' own official sheets** (TDS / SDS / MSDS / RoHS). The internal rule is strict: whenever manufacturer data exists (Polymaker, Prusament, Bambu Lab, eSUN, SUNLU, Eryone…), it **always takes precedence** over any other source. For each material you get the recommended temperatures, density, drying conditions, colour codes, and **direct links to the official TDS / MSDS / RoHS sheets** hosted by the manufacturer itself.

The database **ships ready to use**: the app bundles a full **offline** snapshot, and the **update checker** keeps it current — on first launch it downloads the current database from the server, and later a "check for updates" pulls a newer one whenever it becomes available.

And the database **keeps growing thanks to the community**. When you import a PDF, you can (it is **checked by default, but optional**) share — **anonymously** — only the **manufacturer facts** from that sheet: brand, material, base type, nozzle / bed window, density, link, revision date, sent to Maison Drabiec's moderation queue. Never the PDF, never your file paths, never any personal or machine data. After review, approved entries join the shared database (manufacturer data always keeping priority). That's how the database fills out over time, for everyone's benefit.

> Important: the software **never** re-hosts manufacturer PDFs. It stores the useful facts (temperatures, density…) and a deep link to the original document.

### Filament not in the database yet? PDF import takes over

If your filament isn't in the database yet, the MD Optimiser builds it from its sheet: you **drop the manufacturer PDF** (SDS or TDS) and the profile is produced automatically (the app can also look up the manufacturer's TDS online), just as from the library. This is the **fallback** for new releases; the vast majority of common materials are already in the database.

And the parser doesn't stop at the parameter table. It also reads the **"test specimen" note** — the test conditions many manufacturers state ("all specimens are printed at 210 °C, 80 mm/s, bed 60 °C"). Those values are **authoritative** and override the table's midpoints, because they describe exactly how the manufacturer obtained its mechanical results. Built-in safety: the chosen nozzle temperature always stays below the stated decomposition threshold.

### The 7 PROCESS profiles by project type, generated with the filament

The same click that creates the filament profile also generates its **7 process profiles by project type**. It all starts from the **printer selector at the top of the window**: you pick **brand → model → nozzle** (or "all nozzles") from the **OrcaSlicer family** (57 brands / 326 models: Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…) and the app produces the 7 profiles tuned for that machine. With "all nozzles" the whole set is multiplied by 4 — for the **Snapmaker U1**, for example: 7 types × 4 nozzles = 28 profiles:

- **Fast prototype** — thick layers, top speed, high accelerations
- **Everyday object** — the strength / speed / finish balance
- **Figurine** — fine layers, tight cornering and low acceleration/jerk to suppress vertical artefacts and resonance (VFA)
- **Vase** — spiral mode, single wall
- **Decoration** — surface ironing, refined finish
- **Toy** — reinforced walls, generous infill
- **Mechanical part** — multiple walls, dense infill

Each profile is tuned for **cornering** and **resonance / VFA** (via acceleration and jerk limits) and to stay under the machine's flow ceiling. The functional, larger-footprint types (Everyday object, Toy, Mechanical part) also get a **modest outer brim** to limit warping and plate detachment; the aesthetic types (Figurine, Vase, Decoration) and the Fast prototype get none. (Because the process set is shared, adhesion keys off the **project type**, not the material.) Filament-specific tuning (temperatures, flow, retraction) stays on the filament profile: one shared set of processes plus per-material tuning is all it takes.

The filament profile targets the **chosen printer** (`compatible_printers`): it inherits the **U1-tuned parent** when that's the machine, and the stock "**Generic &lt;polymer&gt;**" of the OrcaSlicer family otherwise. The shared process set carries the cornering / resonance tuning and the fork features.

### Fork features enabled out of the box

The generated profiles activate the **Snapmaker_Orca / OptimusOrca** fork capabilities:

- **Filament economy** — shrinks wipe-tower purges (−30 % by default for a freshly-used nozzle), removes redundant tool changes, and scales extrusion by curvature. On multi-colour prints, savings can reach **up to −15 to −30 %** depending on the part and the number of colour changes.
- **Scarf seams** — a sloped joint that makes the Z-seam nearly invisible on most materials (disabled on TPU, which doesn't ramp cleanly).
- **Colour-mixing readiness** — the safe optimisation (region-collapse) is enabled; experimental modes (gradient, dithering…) stay off by default so single-colour prints are unaffected.

### Always current, lightweight, bilingual

The database receives **regular updates**: the app ships an offline-capable snapshot and the **update checker** pulls a newer database as soon as one is published, with no reinstall. The interface is **bilingual French / English**, the binary is **lightweight** (Tauri, ≈ 10 MB per OS) and runs on **Windows, macOS and Linux**.

---

## 5. Technical specifications

| Specification | Detail |
|---|---|
| Product type | Desktop application (utility for FDM 3D printing) |
| Operating systems | Windows · macOS (Apple Silicon & Intel) · Linux |
| Distribution format | **A single ZIP** with three folders: Windows (**`.exe`**), macOS (**`.dmg`**), Linux (**`.AppImage`**) |
| Target slicers | **Slicer selector**: OrcaSlicer · Bambu Studio · Creality Print · SnapmakerOrca / OptimusOrca · custom folder (profiles written to the chosen slicer's preset folder, resolved per OS; choice remembered) |
| Accepted inputs | **Select one or several materials from the bundled database** (primary flow; brand filter + free-text search); as a fallback for a filament not yet listed: manufacturer spec-sheet PDF (SDS / TDS), with optional online TDS lookup |
| Generated outputs | Filament profile `.json` named "Brand Material" + its 7 process profiles `.json` (by project type, for the chosen printer; × 4 with "all nozzles"). With multi-select: one filament profile per material + a single shared process set |
| Database | 700+ materials (709 entries, 122 brands) built from manufacturers' own official sheets (TDS / SDS / MSDS / RoHS); ships bundled, kept current by the update checker |
| Printers covered | OrcaSlicer family — 57 brands / 326 models (Creality, Bambu, Snapmaker, Anycubic, Prusa…); all their nozzles |
| Interface languages | French · English (switchable, remembered) |
| Technology | Tauri (lightweight binary, ≈ 10 MB per OS) |
| OCR (scanned PDFs) | Supported via system-installed Tesseract (text PDFs work without it) |
| Internet connection | Optional (online TDS lookup and database updates); the core works offline |
| Software licence | Proprietary — strictly personal & private use (see LICENSE.md) |

> Requirements: a Windows 10/11 PC, recent macOS, or Linux machine capable of running the Snapmaker_Orca slicer. No heavy dependencies; OCR is only needed for scanned (image) sheets.

---

## 6. What's included

- The **MD Optimiser** application for your system (Windows / macOS / Linux).
- The **database of 700+ materials** shipped ready to use (temperatures, density, drying, colours, links to official sheets), kept current by the update checker.
- The **one-click flow**: slicer selector (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, custom folder), printer selector (brand → model → nozzle), filament library with a **brand filter** and **multi-select**, then joint generation of the **filament profile** + its **7 process profiles** by project type (up to the full Snapmaker U1 set).
- The **PDF fallback import** (SDS/TDS PDF, optional online TDS lookup) to build a filament not yet in the database.
- The **optional community contribution** (anonymous, checked by default, can be turned off) that shares only the manufacturer facts of an imported sheet to grow the shared database.
- The **update checker** (downloads the database on first launch, a newer one thereafter) and **supports / anti-warp** settings (an outer brim on the functional project types, clean support release).
- **Fork features enabled**: filament economy, scarf seams, colour-mixing readiness.
- The **bilingual FR / EN** interface.
- **Regular database updates**.

---

## 7. SEO

**Keywords**

`filament database` · `Snapmaker U1 filament profile` · `3D printing optimiser` · `one-click OrcaSlicer profiles` · `automatic filament settings` · `3D filament library` · `multicolor filament saving` · `import TDS SDS filament sheet` · `3D printing process profiles` · `Snapmaker_Orca`

**Meta description (≈ 155 characters)**

> Database of 700+ official filaments: pick your printer, choose your material, and generate filament + process profiles for Snapmaker_Orca in one click. Win/macOS/Linux.

---

## 8. FAQ

**Is it compatible with my printer / slicer?**
Yes, for the whole **OrcaSlicer family**: OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca. A **slicer selector** at the top of the window sets where profiles are written (you can also point it at a custom folder), and the choice is remembered. Then comes the **printer selector**: you pick brand, model and nozzle (or "all nozzles") from **their printers** (57 brands, 326 models), and the profiles appear in the slicer's menus once generated. PrusaSlicer (`.ini` format) is planned next.

**Where does the data come from? Is it reliable?**
The database prioritises **official manufacturer sites and sheets**. Whenever manufacturer data exists, it always takes precedence over any other source. For each material you get **links to the official sheets** (TDS / MSDS / RoHS) hosted by the manufacturer. The software never re-hosts those PDFs.

**What is shared with the "community database", and what about my privacy?**
When you import a PDF, a checkbox **"Share this sheet (anonymous) with the community database"** is **checked by default**, but stays entirely **optional** (untick it anytime; it never blocks the import). After a successful import, only the **manufacturer facts** are sent to Maison Drabiec's moderation queue: brand, material, base type, nozzle / bed window, density, link and revision date. **Never** the PDF, **never** your file paths, **no** personal or machine data; the server keeps only a hashed-IP identifier to curb abuse. After review, approved entries join the shared database — manufacturer data always keeping priority.

**How do I find my filament quickly in the database?**
A **"Brand"** dropdown above the search lets you filter by manufacturer, combined with the free-text search (product name or family). You can also **tick several materials** and generate, in one click, one filament profile per material plus a single shared set of the 7 process profiles.

**How is this better than a generic "PLA" profile?**
A generic profile applies averages. The MD Optimiser starts from **your specific filament**: it draws on a **database of 700+ materials** built from manufacturers' own official sheets (and, failing that, reads the parameter table *and* the "test specimen" note from the PDF — the authoritative test conditions) to dial in nozzle, bed, speed and flow precisely, then adds process profiles designed for your project type.

**My PDF is a scan / an image — will it work?**
Yes — for the fallback import of a filament not yet in the database, provided you install **Tesseract** on your system (OCR). Manufacturers' "text" PDFs — the vast majority — work with nothing extra installed.

**Are database updates included?**
Yes. The app ships an offline-capable snapshot; on first launch the update checker downloads the current database, then "check for updates" pulls a newer one whenever it's published, with no need to reinstall the software.

**How much filament can I realistically save?**
On **multi-colour** prints, savings can reach **up to −15 to −30 %** depending on the part and the number of colour changes (purge shrinking, removal of needless tool changes). On single-colour prints the gain comes mostly from curvature-based scaling. These are ballpark figures, not a guaranteed number.

**Refund policy?**
*(To be completed by the owner per the store's terms — e.g. refund within X days if the software fails to launch on the stated configuration.)*

---

## 9. Visual / screenshot ideas to prepare

- **Hero screenshot**: the **filament library** with a search in progress and a material selected, ready for the single click.
- **Global printer selector**: the menu at the top of the window, brand → model → nozzle (with the "all nozzles" option).
- **One-click result**: the filament profile *and* its 7 process profiles generated together and shown (a "Ready" badge).
- **Thumbnails of the 7 project types**: one icon/render per intent (Prototype, Figurine, Vase, Decoration, Toy, Mechanical part, Everyday object).
- **Fallback import**: the "Single PDF" window with a sheet dropped in for a filament not yet in the database, and the resulting profile.
- **"Filament economy" diagram**: purge tower before/after, captioned "up to −15 to −30 % on multi-colour".
- **"Scarf seam" close-up**: comparison of a classic Z-seam vs a scarf seam.
- **OS logo strip**: Windows / macOS / Linux plus "bilingual FR/EN" and "lightweight (Tauri)".
- **"700+ materials" visual**: a brand cloud + key figures (709 entries, 122 brands) with the note "official manufacturer sources".
