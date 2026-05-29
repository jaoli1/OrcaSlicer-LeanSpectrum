# Filament & Print-Profile Optimiser by Maison Drabiec — User Manual

> **Software version:** 0.4.0 · **Language of this document:** English ([Version française](WIKI_FR.md))
>
> A desktop application built around a **filament database** (compiled from manufacturers' own official sheets) that generates, in one click, optimised **filament** and **process** profiles for the **OptimusOrca / Snapmaker_Orca** slicer.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [First launch & interface](#3-first-launch--interface)
4. [The Filament Library (main mode)](#4-the-filament-library-main-mode)
5. [The global printer selector](#5-the-global-printer-selector)
6. [Single PDF (fallback mode)](#6-single-pdf-fallback-mode)
7. [The PROCESS profile library](#7-the-process-profile-library)
8. [Updates](#8-updates)
9. [Using the profiles in OptimusOrca / Snapmaker_Orca](#9-using-the-profiles-in-optimusorca--snapmaker_orca)
10. [Troubleshooting / FAQ](#10-troubleshooting--faq)
11. [Credits & legal](#11-credits--legal)

---

## 1. Introduction

The **Filament & Print-Profile Optimiser by Maison Drabiec** (short name: *Optimiser MD*) is a desktop utility that removes the tedious step of manually tuning a new filament.

At the heart of the app is now a **filament database**. Built from the **manufacturers' own official sheets** (TDS — technical data sheet, SDS / MSDS — safety data sheets, RoHS), it gathers **709 materials** from **122 brands**, with their temperatures, colours and links back to the source documents. It is **bundled offline** (a snapshot is seeded into the app on first run), then **refreshed from the Maison Drabiec server** when you click "Check for updates".

The workflow is simple:

1. you pick your **slicer** in the selector at the top of the window (OrcaSlicer, Bambu Studio, Creality Print, SnapmakerOrca / OptimusOrca, or a custom folder) — that's where the profiles will be written;
2. you pick your **printer** (brand → model → nozzle);
3. you **search for a material** in the database (by brand, name or family: PLA, PETG…), and can **tick several** at once;
4. the app generates, in **a single click**, the filament profile **and** the seven project-type process profiles, tuned for that printer.

In return, the app writes for you:

- a **filament profile** `.json` (nozzle and bed temperatures, density, volumetric flow, vendor…), made compatible with the chosen printer;
- a **library of process profiles** `.json` by project type and nozzle diameter.

These profiles then appear directly in the menus of the **OptimusOrca / Snapmaker_Orca** slicer.

For a filament not yet in the database, a **"Single PDF"** fallback mode lets you import a manufacturer sheet (SDS / TDS) as a PDF.

### Who is it for?

- **Snapmaker U1 owners**: the profiles are tuned for this machine and its four nozzles (0.2 / 0.4 / 0.6 / 0.8 mm).
- More broadly, **FDM 3D-printing users** running the Snapmaker_Orca / OptimusOrca slicer (OrcaSlicer family) who want to start from reliable settings derived from official manufacturer data rather than values picked up on forums.

> **Note**
> The app never re-hosts manufacturer PDFs. The database stores the **useful facts** (temperatures, density, drying, colours…) and a **direct link** to the original document on the manufacturer's site.

---

## 2. Installation

The application ships as a lightweight binary (Tauri technology) for each operating system. The **release is a single ZIP archive** containing three folders: `Windows/` (`.exe`), `MacOS/` (`.dmg`) and `Linux/` (`.AppImage`). Unzip the archive, then open the folder matching your system.

### Windows (`.exe`)

1. Open the `Windows/` folder of the archive and run the `.exe` file.
2. Follow the installer steps.
3. Start the app from the Start menu.

### macOS (`.dmg` — unsigned app)

1. Open the `.dmg` file in the `MacOS/` folder of the archive.
2. Drag the app into the **Applications** folder.
3. **On first launch**, do not double-click: **right-click the app > Open**, then confirm in the dialog.

> **Important**
> The app is **not signed** with a paid Apple developer certificate. macOS will therefore show an "unverified app" warning on first launch. The **right-click > Open** workaround is normal and only needed once. See also the [FAQ](#10-troubleshooting--faq).

### Linux (`.AppImage`)

1. Take the `.AppImage` file from the `Linux/` folder of the archive.
2. Make it executable:

   ```bash
   chmod +x Optimiser-MD-*.AppImage
   ```

3. Run it by double-clicking or from the command line:

   ```bash
   ./Optimiser-MD-*.AppImage
   ```

### Where does the app write the profiles?

The Optimiser writes its profiles into the **user folder of the chosen slicer** (see the [slicer selector](#3-first-launch--interface)), where the slicer reads them. For SnapmakerOrca / OptimusOrca, for example:

- **filament profiles** go into the `filament/` subfolder of your user profile;
- **process profiles** go into the `process/` subfolder.

The other OrcaSlicer-family slicers (OrcaSlicer, Bambu Studio, Creality Print) follow the same layout in their own preset folder; with the "custom folder" option you point directly at the location you want.

> **Important**
> The chosen slicer's user folder must already exist. If it does not, **open the slicer at least once** so it creates your profile folder, then run the generation again.

The app also keeps a **log file** in its own system data folder (useful for support if something goes wrong).

---

## 3. First launch & interface

On first start, the interface opens in your OS language (French or English) and then remembers your choice.

### Language picker (FR / EN)

In the top-right corner, two buttons **EN** and **FR** switch the interface language. Your choice is kept across sessions.

### "Check for updates" button

Below the header is a **"Check for updates"** button with a status area next to it. This is what installs and updates the **filament database**: on first use it downloads the current database, and afterwards it pulls a newer one whenever the server publishes it. See the [Updates](#8-updates) section.

### Slicer selector

At the very top of the window, a **"Slicer"** selector decides which slicer the generated profiles are written into: **OrcaSlicer**, **Bambu Studio**, **Creality Print**, **SnapmakerOrca / OptimusOrca**, or a **custom folder** you point to yourself. The app automatically resolves the chosen slicer's user preset folder for your system (Windows / macOS / Linux), and **remembers** that choice between sessions. See also the [global printer selector](#5-the-global-printer-selector).

### Global printer selector

Right **above the tabs**, a **"Printer"** selector lets you choose **Brand → Model → Nozzle**, with an **"All nozzles"** option to handle every nozzle of the machine at once. This selector is **shared by both libraries** (Filament and process): the printer chosen here drives both the one-click generation and the process-only generation. It covers the whole **OrcaSlicer family** (57 brands / 326 models). See the [global printer selector](#5-the-global-printer-selector) section.

### The 3 tabs

Below the printer selector, the interface is organised into three tabs, in this order:

| Tab | Purpose |
|---|---|
| **Filament Library** | Search a material in the filament database, then generate in one click the filament profile **and** its process profiles for the chosen printer. |
| **Process library** | Generate the set of process profiles by project type (for the chosen printer, or the full Snapmaker U1 set). |
| **Single PDF** | Fallback mode: import a single manufacturer PDF for a filament not yet in the database. |

---

## 4. The Filament Library (main mode)

This is the app's central mode, in the **Filament Library** tab. It links the **filament database** to the profile generator: you pick a material and a printer, and the app writes the filament profile **and** its process profiles in a single click.

> This tab replaces the former "Local database".

### Steps

1. **Pick your slicer**, then your **printer**, in the selectors at the top of the window (see the [global printer selector](#5-the-global-printer-selector) section).
2. **Search for a filament.** You can first narrow the list with the **"Brand"** dropdown placed above the search, then refine in the text box by typing a **product name** or a **family** (PLA, PETG, ABS…). The brand filter and the text search combine; the list filters as you type.
3. **Select one or several materials** in the list (tick several to process them at once). You see the information drawn from the manufacturer sheets: polymer family, **temperature** ranges (nozzle / bed), **density**, available **colours** and links to the source documents.
4. Click **"Generate filament + process"**.
5. The app writes one **filament profile** per selected material and a **shared set of 7 process profiles** by project type, then shows a summary (filament profiles created, number of process profiles, target printer) and a **log**.

### One click = filament + process

From the chosen material and the printer in the global selector, the app generates **together**:

- the **filament profile** (temperatures, flow, retraction from the material), made compatible with the chosen printer;
- the **7 process profiles** by project type (see [The PROCESS profile library](#7-the-process-profile-library)).

The split stays "**one shared process set + per-filament tuning**": the filament-specific tuning (temperatures, flow, retraction) lives on the **filament profile**, while the process profiles carry the print geometry, cornering / resonance and the fork features.

> **Note — the "All nozzles" option**
> If you ticked **"All nozzles"** in the selector, the app generates the **7 process profiles per nozzle** of the machine (for example ×4 for the Snapmaker U1, i.e. 28 profiles), in addition to the filament profile(s).

> **Note — multi-select and profile names**
> If you tick **several materials**, the app creates **one filament profile per material** but a **single shared set** of the 7 process profiles (a shared process set cannot be per-material). Each filament profile is named "**Brand Material**" (e.g. "Eryone PLA+"); legal filename characters such as "+" are preserved.

### What the displayed information contains

The data comes from the manufacturer's official sheets and feeds the profile:

- the **nozzle temperature range** (and the chosen value, at the midpoint of the range);
- the **bed temperature**;
- the **drying** conditions and the **density**;
- the **colours** (with their colour code);
- the **links** to the source sheets (TDS / SDS…).

Fields that had to be **estimated** (because the sheet gave no value) are backfilled from per-polymer-family defaults, then flagged: the profile may then carry a **"Needs review"** badge inviting you to double-check before a critical print. A complete profile carries the **"Ready"** badge.

---

## 5. The global printer selector

At the top of the window, **above the tabs**, the **"Printer"** selector decides which machine the profiles are generated for. It is **shared by the Filament Library and the Process library**. It works hand in hand with the **slicer selector** (see [section 3](#slicer-selector)), which decides *which slicer* the profiles are written into.

### Choosing your machine

1. Pick the **Brand** (for example Snapmaker, Creality, Bambu Lab, Prusa, Anycubic…).
2. Pick the **Model**.
3. Pick the **Nozzle**, or select **"All nozzles"** to generate the profiles for every nozzle of the machine at once.

The catalogue covers the whole **OrcaSlicer family**: **57 brands** and **326 models**.

### Multi-printer: a correct filament profile

The generated filament profile is made **compatible with the chosen printer**:

- for the **Snapmaker U1**, it inherits the U1-tuned parent (the "@U1" chain);
- for **any other** OrcaSlicer-family printer, it inherits the stock **"Generic &lt;polymer&gt;"** profile (e.g. Generic PLA, Generic PETG…).

This way, the filament shows up in the slicer's menu for the selected machine, starting from base settings consistent with it.

---

## 6. Single PDF (fallback mode)

For a filament that is **not yet in the database**, the **Single PDF** tab lets you generate a profile from a manufacturer sheet you provide yourself. It is the last tab of the interface.

### Steps

1. **Drop a PDF** onto the drop zone ("Drop a .pdf here, or click to pick a file"), or click to open the file picker.
2. Keep checked (or not) the option **"Also look for the manufacturer's TDS online (recommended)"**. If your PDF is a plain safety sheet (SDS) without printing data, this option fetches the matching technical sheet (TDS) from the manufacturer's site to complete the profile.
3. Keep checked (or not) the option **"Share this sheet (anonymous) with the community database"**, **checked by default**. After a successful import, it sends only the **manufacturer facts** from the sheet to help grow the shared database (see the [Community contributions](#community-contributions-anonymous-and-optional) subsection below). It is entirely optional and never blocks the import.
4. Click **"Create filament profile"**.
5. The app shows the result (extracted fields, any badges) and a **log** detailing what it did.

### What is extracted

The parser does more than copy a table — it reads the sheet and pulls the genuinely useful parameters:

- the **nozzle temperature** (min–max range and chosen value);
- the **bed temperature**;
- the **drying** conditions and the **density**;
- the **print speed**;
- the **test-specimen note** when present.

> **Note — the test-specimen note is authoritative**
> Many sheets state the exact conditions under which the mechanical-test bars were printed (for example: *"all splines are printed at 210 °C, 80 mm/s, base plate 60 °C"*). When this note exists, its values **override** the midpoints of the parameter table, because they describe precisely how the manufacturer obtained its results.

Fields that had to be **estimated** (because the sheet gave no value) are flagged, and the profile may then carry a **"Needs review"** badge inviting you to double-check before a critical print. A complete profile carries the **"Ready"** badge.

### Community contributions (anonymous and optional)

The filament database **grows over time thanks to shared imports**. On the **Single PDF** tab, the checkbox **"Share this sheet (anonymous) with the community database"** is **checked by default**, but you can untick it at any time.

Concretely, after a **successful import**:

- the app sends **only the manufacturer facts** extracted from the sheet: brand, material, base type, temperature window (nozzle / bed), density, link to the sheet and revision date;
- this data goes to Maison Drabiec's **moderation queue**; after review, approved entries enter the shared database distributed to users;
- **manufacturer data always keeps priority** over any other source.

What is **never** sent: the **PDF** itself, your **file paths**, and any **personal or machine data**. The server keeps only a **hashed-IP identifier** to curb abuse.

> **Note**
> Sharing is **entirely optional** and **never blocks** the import: if you untick the box, the profile is generated in exactly the same way, simply without a contribution.

---

## 7. The PROCESS profile library

The **Process library** tab generates process profiles by **project type** for the printer chosen in the [global selector](#5-the-global-printer-selector) (the whole OrcaSlicer family: Creality, Bambu Lab, Snapmaker, Anycubic, Prusa…): the app produces the 7 profiles tuned for that specific printer. A one-click button also generates the **full Snapmaker U1 set** (7 types × 4 nozzles = 28 profiles).

These are **the same 7 process profiles** that the one-click generation in the [Filament Library](#4-the-filament-library-main-mode) produces; this tab is for regenerating them on their own, without going through a material.

The principle is "**one shared process set + per-filament tuning**": the filament-specific tuning (temperatures, flow, retraction) stays on the **filament profile**, while the process profiles carry the print geometry (layers, walls, infill, speeds, accelerations, finishing).

### The 7 project types × 4 nozzles = 28 profiles

Each project type is generated for the **0.2 / 0.4 / 0.6 / 0.8 mm** nozzles. Layer height automatically tracks the nozzle diameter (clamped between 25% and 75% of the diameter).

Reference values (at the **0.4 mm** nozzle):

| Project type | Layer (0.4) | Walls | Infill | Pattern | Outer-wall speed | Accel / Jerk | Specificity |
|---|---|---|---|---|---|---|---|
| **Fast prototype** (Prototype rapide) | 0.28 mm | 1 | 8% | grid | 150 mm/s | 10000 / 12 | Thick layers, maximum speed and accelerations |
| **Everyday object** (Objet du quotidien) | 0.20 mm | 3 | 15% | grid | 120 mm/s | 6000 / 9 | Balance of strength / speed / finish |
| **Figurine** | 0.12 mm | 3 | 15% | gyroid | 50 mm/s | 2000 / 5 | Fine layers, **tight cornering and low accel/jerk** to kill vertical artefacts and resonance (VFA) |
| **Vase** | 0.20 mm | 1 | 0% | gyroid | 60 mm/s | 4000 / 7 | **Spiral mode**, single wall, hollow part |
| **Decoration** (Décoration) | 0.16 mm | 2 | 10% | lightning | 80 mm/s | 4000 / 7 | **Ironing** of the top surface, polished finish |
| **Toy** (Jouet) | 0.20 mm | 4 | 30% | grid | 100 mm/s | 6000 / 9 | Reinforced walls, generous infill |
| **Mechanical part** (Pièce mécanique) | 0.24 mm | 5 | 45% | grid | 60 mm/s | 4000 / 7 | Multiple walls, dense infill |

> **Note**
> Project-type names appear in the slicer in French (the product language), e.g. `Figurine @U1 (0.4 nozzle)`, `Pièce mécanique @U1 (0.6 nozzle)`. The English labels above are provided for guidance.

### What each profile optimises

- **Layer height** — matched to the nozzle, within the printable 25–75% of diameter window; first layer slightly thicker for adhesion.
- **Walls (wall loops)** and **top / bottom shells** — according to the target strength.
- **Infill** — density and pattern (grid, gyroid, lightning) according to the intent.
- **Speeds** — outer wall, inner wall, infill and top surface.
- **Cornering & resonance / VFA** — via **acceleration** and **jerk** limits: low and tight for the Figurine (fewer fine vertical artefacts), high and fast for the Prototype.
- **Vase (spiral) mode** for the Vase type; **ironing** for Decoration.
- **Scarf seams** — enabled on the surface-quality intents (Everyday object, Figurine, Decoration, Toy, Mechanical part) for nearly invisible Z-seams; disabled on the Vase (single continuous wall) and the Prototype.
- **Filament economy** — enabled by default (purge shrinking to −30%, removal of redundant tool changes, curvature-aware extrusion scaling, forced M83 relative mode).
- **Color-mixing readiness** — the safe *region-collapse* optimisation is enabled; experimental modes stay off by default.

### Supports & adhesion (anti-warping)

The process profiles ship settings designed so **supports grip the plate but peel cleanly off the model**, and to **limit warping and part detachment**:

- **Support → model release**: a vertical gap above supports (`support_top_z_distance` 0.2 mm), a spaced rectilinear interface (`support_interface_spacing` 0.5 mm, `support_interface_pattern` rectilinear), 0.35 mm XY distance — supports lift off by hand without scarring the surface. (This setting is **unchanged** across project types.)
- **Anti-warp by project type**: because the process profiles form a **shared set** (not tied to the material), plate grip keys off the **project type**. The **functional, larger-footprint** types — **Everyday object, Toy, Mechanical part** — get a **modest outer brim** (`brim_type` outer_only) to limit warping and detachment. The **aesthetic** types — **Figurine, Vase, Decoration** — and the **Fast prototype** get none, to avoid extra cleanup or marking the part.

> These only take effect when the slice generates supports / a brim; they are conservative starting values you can fine-tune. (Research: `data/RESEARCH_supports_adhesion.md`.)

### Generating the profiles

**For any printer** (on-demand):
1. Pick your **brand**, **model** and **nozzle** in the [global printer selector](#5-the-global-printer-selector) at the top of the window. (With **"All nozzles"**, the profiles are generated for every nozzle of the machine.)
2. Open the **Process library** tab.
3. Click **"Generate process for the selected printer"** — the app writes the **7 profiles** (one per project type) into the `process/` folder of your user profile, inheriting that printer's stock base process (the OrcaSlicer → SnapmakerOrca chain).

**Snapmaker U1 shortcut**: the **"Generate the Snapmaker U1 set"** button produces all **28 profiles** (7 types × 4 nozzles) for the U1 directly.

> **Important**
> If the Snapmaker_Orca user folder is not found, open OptimusOrca once so it creates your profile folder, then run the generation again.

---

## 8. Updates

The Optimiser clearly separates **two things**: the filament **database** (just data) and the **application** itself (the binary).

The database is **bundled**: an offline snapshot is seeded into the app's data folder on first run, so the Filament Library works without a connection. On the **first use** of the button, the app downloads the current database from the server; afterwards, it only pulls a new database if the server publishes a newer one.

### Manual and automatic checks

- An **automatic check** runs **at launch**.
- You can also trigger a check at any time with the **"Check for updates"** button.

### What happens in each case

- **A newer database is published** → it is **downloaded automatically** into the app's data folder (it is only data), and its version is remembered. A message confirms the database update.
- **A newer version of the app is published** → it is **offered for download** (the interface shows the available version and a **"Download"** button that opens the download page). Distribution is a ZIP archive: **the binary is never silently replaced**.
- **Everything is up to date** → a "You already have the latest version" message is shown.

> **Note**
> If the check fails (offline, server down), the app reports it without claiming an update is available.

---

## 9. Using the profiles in OptimusOrca / Snapmaker_Orca

Once generated, the profiles are written to the slicer's user folder and appear in its menus.

### Where do they appear?

- The **filament profile** appears in the slicer's **Filament menu**, in the user-profile section.
- The **process profiles** appear in the **Process menu**, named by project type and nozzle (for example `Figurine @U1 (0.4 nozzle)`).

### How to select them

1. Open **OptimusOrca / Snapmaker_Orca** (restart it if it was already open during generation — see the [FAQ](#10-troubleshooting--faq)).
2. Select the **printer** (and nozzle diameter) you had chosen in the global selector at generation time.
3. In the **Filament** menu, choose the filament profile generated for your spool.
4. In the **Process** menu, choose the profile matching your **project type** and **nozzle**.

> **Note**
> The process profile and the filament profile complement each other: the filament carries the temperatures and flow, the process carries the print geometry and the fork features. Choose both for an optimal result.

---

## 10. Troubleshooting / FAQ

### A profile does not appear in the slicer menu

The slicer reads its user-profile folder **only at startup**. If you just generated a profile while the slicer was open, **close and restart OptimusOrca / Snapmaker_Orca**: the profile will then appear in the Filament or Process menu.

### macOS shows "unverified app" / "not verified by Apple"

This is expected: the app is not signed with a paid Apple certificate. Instead of double-clicking, **right-click the app > Open**, then confirm. This is only needed on first launch.

### "Snapmaker_Orca user folder not found"

Generation needs the slicer to have already created its profile folder. **Open OptimusOrca / Snapmaker_Orca at least once**, then run the generation again (filament profile or process library).

### The database is not found / the update fails

Updating the database requires an **Internet connection**. If the check fails (offline, server temporarily down), the app reports it without blocking its other features. Try again later with **"Check for updates"**. The core of the app (extraction, profile generation) works **offline**.

### My PDF is a scan / an image

Direct text extraction works for the vast majority of manufacturer sheets ("text" PDFs). For a purely image-based PDF (a scan), character recognition (OCR) is needed; it requires installing **Tesseract** on the system.

### Online TDS lookup finds nothing

The "look for the TDS online" option depends on the links present on the manufacturer's page. If no additional technical sheet is found, the app mentions it in the log and keeps the data already extracted from the PDF.

---

## 11. Credits & legal

- **Licensing.** The **Filament & Print-Profile Optimiser by Maison Drabiec** is **proprietary** software, for strictly **personal & private use** (see `LICENSE.md`). The target slicer **OptimusOrca / Snapmaker_Orca**, by contrast, remains **open-source** software distributed under the **AGPL-3.0-or-later** license: the two licenses are separate.
- **Filament data.** The data comes from the **manufacturers' official sites**. When a manufacturer value exists, it takes precedence over any other source. The app **does not re-host** manufacturer PDFs: it stores the useful facts and, where relevant, a link back to the original document. Any **community contributions** (see [section 6](#community-contributions-anonymous-and-optional)) are anonymous and limited to manufacturer facts.
- **Brand.** "Filament & Print-Profile Optimiser by Maison Drabiec" and the MD monogram are the trademark of Maison Drabiec.

> Public standards (GHS, ISO 11014-1) are public references; no proprietary schema or profile content is reproduced.
