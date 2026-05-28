# Filament & Print-Profile Optimiser by Maison Drabiec — User Manual

> **Software version:** 0.1.17 · **Language of this document:** English ([Version française](WIKI_FR.md))
>
> A desktop application that turns a manufacturer data sheet (SDS/TDS PDF) or a catalog URL into optimised **filament** and **process** profiles for the **OptimusOrca / Snapmaker_Orca** slicer.

---

## Table of contents

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [First launch & interface](#3-first-launch--interface)
4. [Mode 1 — Import a PDF data sheet](#4-mode-1--import-a-pdf-data-sheet)
5. [Mode 2 — Vendor catalog](#5-mode-2--vendor-catalog)
6. [Mode 3 — Local database](#6-mode-3--local-database)
7. [The PROCESS profile library](#7-the-process-profile-library)
8. [Updates](#8-updates)
9. [Using the profiles in OptimusOrca / Snapmaker_Orca](#9-using-the-profiles-in-optimusorca--snapmaker_orca)
10. [Troubleshooting / FAQ](#10-troubleshooting--faq)
11. [Credits & legal](#11-credits--legal)

---

## 1. Introduction

The **Filament & Print-Profile Optimiser by Maison Drabiec** (short name: *Optimiser MD*) is a small desktop utility that removes the tedious step of manually tuning a new filament.

You give it one of three inputs:

- a **manufacturer PDF** (SDS — safety data sheet, or TDS — technical data sheet);
- the **URL of a vendor's catalog / certificates page**;
- a **local folder** of already-downloaded PDFs.

In return, the app writes for you:

- a **filament profile** `.json` (nozzle and bed temperatures, density, volumetric flow, vendor…);
- a **library of process profiles** `.json` by project type and nozzle diameter.

These profiles then appear directly in the menus of the **OptimusOrca / Snapmaker_Orca** slicer.

### Who is it for?

- **Snapmaker U1 owners**: the profiles are tuned for this machine and its four nozzles (0.2 / 0.4 / 0.6 / 0.8 mm).
- More broadly, **FDM 3D-printing users** running the Snapmaker_Orca / OptimusOrca slicer who want to start from reliable settings derived from official manufacturer data rather than values picked up on forums.

> **Note**
> The app never re-hosts manufacturer PDFs. It extracts the useful facts (temperatures, density, drying…) and, where relevant, keeps a link back to the original document.

---

## 2. Installation

The application ships as a lightweight binary (Tauri technology) for each operating system.

### Windows (`.exe` / `.msi`)

1. Download the Windows installer.
2. Run the installer and follow the steps.
3. Start the app from the Start menu.

> **Note**
> The `.msi` format is recommended on Windows; an `.exe` (NSIS) installer is also provided.

### macOS (`.dmg` — unsigned app)

1. Open the downloaded `.dmg` file.
2. Drag the app into the **Applications** folder.
3. **On first launch**, do not double-click: **right-click the app > Open**, then confirm in the dialog.

> **Important**
> The app is **not signed** with a paid Apple developer certificate. macOS will therefore show an "unverified app" warning on first launch. The **right-click > Open** workaround is normal and only needed once. See also the [FAQ](#10-troubleshooting--faq).

### Linux (`.AppImage`)

1. Download the `.AppImage` file.
2. Make it executable:

   ```bash
   chmod +x Optimiser-MD-*.AppImage
   ```

3. Run it by double-clicking or from the command line:

   ```bash
   ./Optimiser-MD-*.AppImage
   ```

> **Note**
> `.deb` and `.rpm` packages may also be provided for the matching distributions.

### Where does the app write the profiles?

The Optimiser writes its profiles into the **Snapmaker_Orca user folder**, where the slicer reads them:

- **filament profiles** go into the `filament/` subfolder of your Snapmaker_Orca user profile;
- **process profiles** go into the `process/` subfolder.

> **Important**
> The Snapmaker_Orca user folder must already exist. If it does not, **open OptimusOrca / Snapmaker_Orca at least once** so it creates your profile folder, then run the generation again.

The app also keeps a **log file** in its own system data folder (useful for support if something goes wrong).

---

## 3. First launch & interface

On first start, the interface opens in your OS language (French or English) and then remembers your choice.

### Language picker (FR / EN)

In the top-right corner, two buttons **EN** and **FR** switch the interface language. Your choice is kept across sessions.

### "Check for updates" button

Below the header is a **"Check for updates"** button with a status area next to it. See the [Updates](#8-updates) section.

### The 4 tabs

The interface is organised into four tabs:

| Tab | Purpose |
|---|---|
| **Single PDF** | Import a single manufacturer PDF (drag-and-drop or file picker). |
| **Vendor catalog** | Paste a "certificates / downloads" page URL and batch-import the detected PDFs. |
| **Local database** | Scan a folder of PDFs already present on the machine. |
| **Process library** | Generate the set of 28 process profiles by project type in one click. |

The first three tabs produce a **filament profile**; the fourth produces the **process profiles**.

---

## 4. Mode 1 — Import a PDF data sheet

This is the most direct mode, in the **Single PDF** tab.

### Steps

1. **Drop a PDF** onto the drop zone ("Drop a .pdf here, or click to pick a file"), or click to open the file picker.
2. Keep checked (or not) the option **"Also look for the manufacturer's TDS online (recommended)"**. If your PDF is a plain safety sheet (SDS) without printing data, this option fetches the matching technical sheet (TDS) from the manufacturer's site to complete the profile.
3. Click **"Create filament profile"**.
4. The app shows the result (extracted fields, any badges) and a **log** detailing what it did.

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

---

## 5. Mode 2 — Vendor catalog

In the **Vendor catalog** tab, you process several sheets at once from a web page.

### Steps

1. **Paste the URL** of a manufacturer's "certificates" or "downloads" page into the field.
2. Click **"Discover PDFs"**. The app fetches the page and **lists every SDS / TDS PDF** it can identify, with a type badge (SDS / TDS / unknown).
3. Tick the documents you want. The **"Select all"** / **"Select none"** buttons make selection easier.
4. You can enable **"Also try to fetch related TDS for each downloaded PDF"** to complete each sheet.
5. Click **"Import selected"**. A progress bar tracks the work, and a summary reports how many profiles were created and any errors.

> **Note**
> Batch import is robust: if one PDF in the selection has a problem, it is reported as an error but does not stop the processing of the others.

---

## 6. Mode 3 — Local database

In the **Local database** tab, you work from PDFs already present on your disk.

### Steps

1. The path field is pre-filled with a **default folder** under your *Downloads* directory (a corpus folder). Change it if your collection lives elsewhere.
2. Click **"Scan folder"**.
3. The app lists the PDFs it found, **grouped by brand** (subfolder).
4. Click a PDF to import it and generate its filament profile.

> **Note**
> The scan explores one level of nesting: `folder/brand/*.pdf` and `folder/brand/product/*.pdf`. Deeper trees are not walked.

---

## 7. The PROCESS profile library

The **Process library** tab generates a **shared set of 28 process profiles**, organised by **project type** and produced for the **4 nozzle diameters** of the Snapmaker U1.

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

### "Generate process library" button

1. Open the **Process library** tab.
2. Click **"Generate process library"**.
3. The app writes the **28 profiles** into the `process/` folder of your Snapmaker_Orca user profile and shows the destination folder.

> **Important**
> If the Snapmaker_Orca user folder is not found, open OptimusOrca once so it creates your profile folder, then run the generation again.

---

## 8. Updates

The Optimiser clearly separates **two things**: the filament **database** (just data) and the **application** itself (the binary).

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
2. Select your **Snapmaker U1 printer** at the correct nozzle diameter.
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

- **Target slicer.** The profiles are intended for **OptimusOrca / Snapmaker_Orca**, an open-source slicer. The slicer is distributed under the **AGPL-3.0-or-later** license.
- **Filament data.** The data comes from the **manufacturers' official sites**. When a manufacturer value exists, it takes precedence over any other source. The app **does not re-host** manufacturer PDFs: it stores the useful facts and, where relevant, a link back to the original document.
- **Brand.** "Filament & Print-Profile Optimiser by Maison Drabiec" and the MD monogram are the trademark of Maison Drabiec.

> Public standards (GHS, ISO 11014-1) are public references; no proprietary schema or profile content is reproduced.
