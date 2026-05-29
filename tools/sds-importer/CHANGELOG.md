# Changelog

All notable changes to the **Custom Filament Profile Creator** (formerly
"LeanSpectrum SDS Importer") are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow
[SemVer](https://semver.org/).

> The directory `tools/sds-importer/` is retained for git-history
> continuity but the product, binary, and bundle identifiers are now
> `Custom Filament Profile Creator` (FR: *Créateur de profils filament
> adaptés*). Tag pattern: `profile-creator-v*` (legacy `sds-importer-v*`
> still triggers the workflow for backward compatibility).

## [0.3.2] — Noms de profils corrigés

- **Noms de profils** : les caractères légaux dans un nom de fichier (`+`, `(`,
  `)`, `,`…) sont désormais conservés — « Eryone PLA+ » ne devient plus « Eryone
  PLA_ ». Seuls les caractères réellement interdits (`\\ / : * ? " < > |`) sont
  remplacés.
- **Re-génération** : régénérer un filament **remplace** son profil au lieu
  d'empiler des copies « (1) », « (2) » dans le slicer.
- **Base** : correction d'une coquille de libellé (« Silk Raibow PLA » → « Silk
  Rainbow PLA »).

## [0.3.1] — Anti-warping par type de projet

- **Adhérence / anti-warping** : brim externe modéré (≈ 5 mm) ajouté aux profils
  process des types fonctionnels à grande empreinte (Objet du quotidien, Jouet,
  Pièce mécanique) pour limiter le warping et le décollement du plateau ; aucun
  brim sur les types esthétiques (Figurine, Vase, Décoration) ni sur le Prototype
  rapide (où il abîmerait la base). Le détachement propre des supports reste
  actif. Un process partagé ne pouvant pas être par-matière, l'adhérence suit le
  type de projet.

## [0.3.0] — Multi-sélection, choix du slicer, nommage propre

- **Choix du slicer** : sélecteur en haut (OrcaSlicer · Bambu Studio · Creality
  Print · SnapmakerOrca/OptimusOrca · dossier personnalisé). Les profils sont
  écrits dans le dossier utilisateur du slicer choisi (chemin résolu par OS), et
  le choix est mémorisé.
- **Multi-sélection de filaments** : la Bibliothèque Filament permet de cocher
  plusieurs matériaux ; un clic génère un profil filament par matériau + un seul
  jeu de 7 profils process partagé.
- **Nommage propre** : le profil filament s'appelle « Marque Matériau » (ex.
  « Eryone ABS CF ») — fini le préfixe polymère redondant et le suffixe
  « (fabricant) ».
- **Cohérence des process** : suppression de l'ancien process « … Scarf @U1 » par
  filament ; l'import PDF (onglet PDF unique) génère désormais filament + les 7
  profils process par type de projet, comme le flux un-clic.
- Bouton « Générer le jeu Snapmaker U1 » retiré (couvert par l'option « toutes les
  buses » du sélecteur de buse).

## [0.2.0] — Bibliothèque Filament + génération filament & process en un clic

- **Bibliothèque Filament** : nouvel onglet relié à la base de données de
  filaments (construite depuis les fiches TDS / SDS / MSDS / RoHS officielles des
  fabricants — 709 matériaux, 122 marques). Recherche par marque / nom / famille,
  badges température et couleurs. La base est **embarquée** (instantané hors-ligne,
  posée à la première utilisation) puis **rafraîchie depuis le serveur** par le
  vérificateur de mise à jour.
- **Sélecteur d'imprimante global** : Marque → Modèle → Buse (ou *toutes les
  buses* de la machine), partagé par les deux bibliothèques.
- **Un clic = filament + process** : depuis un matériau choisi + l'imprimante
  choisie, génère le profil filament **et** les 7 profils process par type de
  projet d'un coup (× le nombre de buses si « toutes les buses »). Le réglage
  propre au filament reste sur le profil filament.
- **Multi-imprimante** : le profil filament cible l'imprimante choisie
  (`compatible_printers`) ; il hérite du parent « @U1 » réglé pour le Snapmaker U1,
  sinon du « Generic <polymère> » de la famille OrcaSlicer.
- **Onglets réorganisés** : la « Base de données locale » devient la Bibliothèque
  Filament, le « Catalogue fabricant » (outil de construction de la base) est
  retiré du client, et « PDF unique » passe en dernier (filament de secours pour
  un matériau pas encore dans la base).

## [0.1.19] — supports & anti-warping

- Supports calibrés pour bien adhérer au plateau mais se **détacher proprement
  du modèle** ; réglages d'adhérence / anti-warping **adaptés à la matière**
  (bordure, bouclier anti-courant d'air, vitesse de 1re couche).

## [0.1.18] — imprimantes universelles

- Génération des profils process pour **toute imprimante de la famille
  OrcaSlicer** (catalogue 57 marques / 326 modèles), à la demande par
  marque → modèle → buse.

## [0.1.17] — vérificateur de mise à jour + ZIP client multi-OS

- **Mises à jour** : bouton « Rechercher une mise à jour » + vérification
  automatique au lancement. La base de filaments se met à jour automatiquement
  quand une version plus récente est publiée sur le serveur ; une nouvelle
  version de l'application est proposée au téléchargement (pas de remplacement
  silencieux du binaire). Sinon : « vous avez déjà la dernière version ».
- **Distribution** : la release produit un ZIP client unique avec trois dossiers
  `Windows/` (.exe), `MacOS/` (.dmg), `Linux/` (.AppImage).

## [0.1.16] — bibliothèque de profils process par type de projet

- **Bibliothèque process partagée** : 7 types de projet (Prototype rapide, Objet
  du quotidien, Figurine, Vase, Décoration, Jouet, Pièce mécanique) × 4 buses
  (0.2 / 0.4 / 0.6 / 0.8) = 28 profils, calibrés couche / murs / remplissage /
  vitesses + cornering & résonance (accélération & jerk), mode vase, repassage,
  coutures scarf et économie de filament. Nouvel onglet « Bibliothèque process »
  + bouton de génération. Les profils héritent des process de base U1 par buse
  (chaîne OrcaSlicer → SnapmakerOrca).

## [0.1.15] — rebrand: Optimisateur de filament et de profils d'impression by Maison Drabiec

The app is rebranded to **Optimisateur de filament et de profils d'impression
by Maison Drabiec** (formerly "Custom Filament Profile Creator"), with a new
**MD monogram** icon (amber-on-dark, the Maison Drabiec palette).

- `productName`, window title, in-app header (FR + EN i18n), bundle descriptions,
  and the landing-page companion section all updated to the new name.
- New `identifier` `fr.maisondrabiec.optimisateur`.
- New icon set generated by `scripts/gen_md_logo.py` (committed, reproducible):
  `icons-source.png` + `icons/{icon.png,icon.ico,32x32,128x128,128x128@2x}`,
  and an in-app header logo (`frontend/md-logo.png`).

No functional change to extraction or profile generation — naming + branding
only. The internal crate name is unchanged to keep the lib/test references stable.

## [0.1.14] — feat: the generated process profile now activates the fork's own features

The companion **process** profile was scarf-only. It now ships every import with
a *fork-aware* process that turns on the three Snapmaker_Orca-fork capabilities
the user asked for — seams, filament economy, and color mixing — all of which are
PROCESS-domain settings that a *filament* profile would silently ignore.

Verified against the fork's C++ so these are not dead keys:
- `filament_economy_*` and `mixed_filament_*` are members of `PrintConfig` (the
  process aggregate; `mixed_filament_*` are gated on `Preset::TYPE_PRINT` in the
  GUI), and `FilamentEconomy::Settings::from_config()` reads them from
  `full_print_config()`, which folds in the active **process** preset — so a
  value set in the generated process genuinely reaches the post-processor.

What the process now carries:
- **Seams** — scarf-joint keys per polymer (unchanged from v0.1.11).
- **Print speed** — the TDS speed on the wall/infill speeds (from v0.1.13).
- **Filament economy** — `filament_economy_enable`, `…_remove_noop_swaps`,
  `…_shrink_purge` (+ `…_shrink_purge_pct=30`), `…_curvature_lh`, `…_force_m83`.
  Enabled to match the fork's own defaults; benefits single-color (curvature-aware
  E scaling) and multi-color (purge shrinking + no-op tool-change removal for
  FullSpectrum) prints alike. `…_merge_travel` stays off (experimental).
- **Color mixing** — `mixed_filament_region_collapse=1` (the safe optimisation).
  The experimental gradient / dithering / pointillism / bias modes are
  intentionally left at their off defaults so single-color prints are unaffected;
  opt into them from Process ▸ Others.

Because filament economy + color-mixing readiness always apply, **every** import
now gets a process companion (named `… Scarf @U1` when scarf is enabled, else
`… Tuned @U1`) — previously a non-scarf, no-speed polymer got none. Three
process tests updated/added (10 profile tests, 46 total, all pass).

## [0.1.13] — feat: honour the TDS print speed + manufacturer test-specimen conditions

Three data-fidelity gaps closed, applied **generically to any filament** (not
just the ERYONE PLA+ that surfaced them):

- **Print speed is no longer dropped.** "Printing speed" (e.g. 30–100 mm/s) is
  a PROCESS-domain setting, so — like the scarf keys before v0.1.11 — it had
  nowhere to live in a filament profile and was silently discarded. The
  companion **process** profile now injects it into `outer_wall_speed`,
  `inner_wall_speed`, `sparse_infill_speed` and `internal_solid_infill_speed`.
  A companion is now generated whenever scarf seams **or** a print speed exist
  (a speed-only companion is named `… Tuned @U1 (0.4 nozzle)`).
- **Manufacturer test-specimen note is now authoritative.** Many TDS state the
  exact conditions their mechanical-test bars were printed at — ERYONE Part III:
  *"All splines are printed under the following conditions: printing
  temperature=210 °C, printing speed=80 mm/s, base plate 60 °C"*. The parser
  now reads that note and uses those values to **override** the parameter-table
  midpoints (nozzle 210 instead of the 190–220 midpoint 205, bed 60, speed 80).
- **Bed temperature picks a sensible value.** Priority: the specimen-note bed
  temp → the rounded midpoint of the recommended range → the range low end.
  Through v0.1.12 it always used the low end (55 °C for ERYONE, vs the 60 °C the
  vendor actually printed at).

Plus two extraction polish fixes:
- **Revision date (MM/YYYY).** The header date (ERYONE "08/2024") is captured
  into `_leanspectrum_metadata.revision_date` instead of staying `null`.
- **De-glued manufacturer.** pdf-extract drops the space before a legal-form
  suffix ("TechnologyCo,.Ltd"); the name is repaired to "Technology Co,.Ltd",
  which also feeds `filament_vendor`.

Eight new/updated unit tests cover the specimen-note override, the speed-only
companion, `effective_print_speed` priority, the de-glue, and the date scan.

## [0.1.12] — fix: generated profiles now actually appear in the slicer

THE core bug. Through v0.1.11 the generated filament (and scarf process)
JSON files were written to `…/Snapmaker_Orca/user/default/…` correctly,
but **never appeared in the slicer's dropdowns** — the whole point of the
app. A 3-agent audit of the OrcaSlicer C++ preset loader found the cause:

- **Missing `version` key (fatal, silent).** `PresetCollection::load_presets`
  → `Preset.cpp` ~L1220: `if (!version) continue;`. Any user preset whose
  `version` is absent or not a parseable Semver is **silently dropped** at
  startup — no error, no dialog, it just doesn't show. Our profiles had no
  `version`.
- **Missing explicit `compatible_printers`.** Relying on inherited
  compatibility wasn't enough; the dropdown filters by a name-match against
  the active printer.
- **Missing `is_custom_defined`.** Acts as a safety net so the preset still
  loads as a user root even if the `inherits` parent can't be resolved.

Fix — both `build_profile_json` (filament) and `build_process_json` (scarf
companion) now emit:
- `"version": "01.10.01.70"` (matches the fork's `SLIC3R_VERSION`; must be
  ≤ the running slicer to avoid a forward-compat migration pass),
- `"is_custom_defined": "1"`,
- `"compatible_printers": ["Snapmaker U1 (0.4 nozzle)"]`.

Verified end-to-end on a real machine: after this change the generated
"PLA — Eryone PLA+ …" filament appears in the slicer's filament dropdown
under the user section and is selectable; the "… Scarf @U1" process likewise
appears in the process dropdown. Two unit tests updated to pin all three
registration keys on both the filament and the process preset.

If you have profiles generated by ≤v0.1.11 that don't show up, just
re-generate them with v0.1.12 (or add the three keys above by hand).

## [0.1.11] — feat: scarf seams actually apply (companion process profile)

v0.1.10 removed the dead `seam_*` / `scarf_*` keys from the filament
profile because they're PROCESS-domain settings the slicer ignores in a
filament profile. This release makes the scarf feature *real*: alongside
the filament profile, the importer now writes a companion **process**
profile that carries the scarf overrides where they actually take
effect.

What gets generated, per import (when scarf is enabled for the polymer):
- A process profile named `<product> Scarf @U1 (0.4 nozzle)` written to
  `…/Snapmaker_Orca/user/<id>/process/`.
- It `inherits` the stock `0.20 Standard @Snapmaker U1 (0.4 nozzle)` and
  overrides ONLY the scarf keys, so all other process settings stay at
  the U1 standard:
  - `seam_slope_type: external` · `seam_slope_conditional: 1`
  - `scarf_angle_threshold: 155` · `scarf_joint_speed: 50%`
  - `scarf_joint_flow_ratio: 1` (coFloat ratio — the old code wrongly
    emitted `100%` for this float key) · `seam_slope_min_length: 20` ·
    `seam_slope_steps: 10`
- Values use the process-profile wire format (plain string scalars and
  `0`/`1` bools), verified against PrintConfig.cpp and the shipped
  profiles — NOT the filament profile's string-array format.

The companion is surfaced as the import's *recommended process*, and
the log tells the user to pick it in the Process dropdown. TPU (scarf
disabled — rubber doesn't ramp cleanly) gets no companion. Two new unit
tests pin the companion schema and the disabled-polymer skip.

To use: after import, select your filament as usual AND choose the
`… Scarf @U1` process — the two together give nearly-invisible Z-seams.

## [0.1.10] — fix: broken profile inheritance + incomplete bed temps

Auditing the generated profile against the real Snapmaker_Orca filament
schema (the "Réglages des matériaux" dialog + the shipped stock
profiles) surfaced four schema bugs that made the output profile
inherit incorrectly or carry dead keys. None of them crashed, but they
silently degraded the result.

1. **`inherits` pointed at 9 non-existent parents (P0).** Only
   `Snapmaker PLA SnapSpeed @U1` (PLA) actually existed in the shipped
   profile set. The other ten polymers inherited from names that don't
   exist — `Snapmaker PETG HF @U1`, `Generic ABS @U1`, `Generic PA @U1`,
   `Generic PC @U1`, `Generic HIPS @U1`, `Generic PP @U1`, … When a
   filament profile's `inherits` can't be resolved, the slicer drops the
   parent and the filament falls back to bare defaults. Fixed: every
   target is now a profile that ships AND is compatible with
   `Snapmaker U1 (0.4 nozzle)`:
   - PLA → Snapmaker PLA SnapSpeed @U1
   - PETG → Snapmaker PETG @U1 · ABS → Snapmaker ABS @U1 ·
     ASA → Snapmaker ASA @U1 · TPU → Snapmaker TPU @U1
   - PC → Generic PC · PA6/PA12 → Generic PA (U1-compatible, no @U1 leaf)
   - HIPS → Snapmaker ABS @U1 (no HIPS profile; HIPS≈ABS) ·
     PP → Snapmaker PETG @U1 (no PP profile; nearest mid-temp leaf)

2. **Bed temperature reached only one plate type (P1).** We set
   `hot_plate_temp` only. The U1's default build plate is **textured
   PEI** (`textured_plate_temp`), so the extracted bed temperature
   silently never applied on the default plate. Fixed: the bed
   temperature is now written to all four plate types and their
   initial-layer variants (`hot_/cool_/eng_/textured_plate_temp`
   ×2).

3. **Glass-transition temperature was extracted but never emitted (P2).**
   We pull the Vicat / Tg value (54 °C for the Eryone PLA+) but only
   stored it in metadata. It's now written to the real
   `temperature_vitrification` key the dialog exposes.

4. **~10 dead process-domain keys in the filament profile (P2).** The
   `seam_*` / `scarf_*` keys we injected are PROCESS settings — no stock
   *filament* profile carries them and the slicer ignores them inside a
   filament profile, so the scarf seams were never actually applied.
   They're removed from the top level (the per-polymer scarf values stay
   in `_leanspectrum_metadata` for reference and a future process-profile
   companion). Proper scarf application needs a process profile — tracked
   as a follow-up.

Also hardened: data-driven keys are now emitted **only when a value is
present**. Previously a missing value wrote `[""]`, which could
overwrite the inherited parent's nozzle/bed temperature with an empty
string. Three new unit tests pin the inherit map, the full PLA schema,
and the empty-value guard.

Identifier unchanged → upgrades over v0.1.9 in place.

## [0.1.9] — fix: nozzle temp lost + manufacturer "Unknown" on real Eryone TDS

v0.1.8 stopped the crash, but the profile it produced from the real
ERYONE PLA+ TDS was incomplete: **nozzle temperature missing**,
**manufacturer "Unknown"**, product name reduced to a bare polymer.
Diagnosed by dumping the exact `pdf-extract` output of the actual PDF
(not a hand-built fixture).

Two layout realities of that file broke the heuristics:

1. **Unit glyph BETWEEN the range numbers.** The nozzle row extracts
   as `Nozzle temperature 190℃-220℃` — the `℃` clings to each number.
   The bed row is `Bed temperature 55-70℃` with the unit only at the
   end. `RANGE_RX` was `(\d+)\s*(-|to|…)\s*(\d+)`: it matched `55-70`
   (unit trailing) but **not** `190℃-220` because `℃` (or `°C` after
   normalisation) sits between the first number and the dash, where the
   regex expected only whitespace. Result: bed extracted, nozzle lost.
   - Fix: `RANGE_RX` (tds.rs) and `TEMP_RANGE_RX` (sds.rs) now skip an
     optional inline unit `(?:°\s*[cf]|℃|℉|°)?` between the first number
     and the separator. Both the raw glyph and the normalised `°C` form
     are accepted.

2. **Glued legal suffix.** pdf-extract drops the space, so the company
   line is `Shenzhen Eryone TechnologyCo,.Ltd`. `MANUFACTURER_RX`
   required a word boundary `\b` immediately before the legal form,
   which doesn't exist between "Technology" and "Co" → no match →
   "Unknown" manufacturer, and (because the brand prefix is derived from
   the manufacturer) a bare `PLA+` product name.
   - Fix: the distinctive multi-char / punctuated forms (Co Ltd, GmbH,
     Inc, LLC, Limited, B.V., …) may now be glued to the preceding word.
     The ambiguous two-letter forms (AG, KG, Oy, AB) keep their leading
     `\b` so they don't match inside ordinary words.

With both fixes the ERYONE PLA+ TDS now yields: nozzle 190-220 °C, bed
55-70 °C, print speed 30-100 mm/s, density 1.23 g/cm³, Vicat 54 °C,
manufacturer "Shenzhen Eryone …", product name "Eryone PLA+", and the
profile is no longer flagged *Needs review*.

A new test `parses_real_eryone_pdf_extract_text` pins the whole chain
against the verbatim pdf-extract output, run through `normalize_unicode`
exactly as the live import does. A `pdf::dump_pdf_text` (`#[ignore]`)
helper was added for future "this PDF extracts weird" diagnosis:
`DUMP_PDF=/path cargo test --release dump_pdf_text -- --ignored --nocapture`.

Identifier unchanged → upgrades over v0.1.8 in place.

## [0.1.8] — fix: window closed silently on PDF import (UTF-8 panic)

The 0.1.7 release installed and launched correctly, but clicking
"Créer le profil filament" after dropping a PDF closed the application
window immediately. No error dialog, no log entry. Tested PDF:
ERYONE PLA+ TDS, which contains 9× `℃` (U+2103, 3 bytes UTF-8) plus
`°` (U+00B0, 2 bytes), fullwidth `）` (U+FF09), fullwidth `，` (U+FF0C),
and en-dashes.

**Root cause** (three senior-agent audits converged): the TDS / SDS
parsers slice the extracted text by **byte** index — `&text[idx..idx+200]`
to look at a 200-byte window after a label, `&text[..text.len().min(1500)]`
for the header, etc. When a bound lands inside a multi-byte UTF-8
sequence, Rust panics `"byte index N is not a char boundary"`. The
panic propagates out of the Tauri command worker thread, kills the
WebView2 host, and the window vanishes — exactly the symptom the user
saw, with no surviving stderr trace because the host died too fast for
env_logger to flush.

Fixes:

- **`text_utils.rs` (new module)**: `safe_slice(s, start, end)` clamps
  both bounds to the string length and snaps `start` DOWN / `end` UP
  to the nearest valid char boundary. Never panics. Comprehensive
  tests cover empty input, pure ASCII, U+2103 ℃, U+00B0 °, and the
  exact "label window after a degree sign" pattern from `tds.rs`.
- **`normalize_unicode(text)` (same module)**: folds `℃ → °C`, `℉ → °F`,
  en-dash / em-dash → ASCII hyphen, fullwidth parens / comma / colon /
  semicolon → ASCII, non-breaking space → space. Runs once on the
  extracted text before either parser sees it, so the regexes no
  longer have to carry per-pattern Unicode classes and the slice
  surface area shrinks.
- **`tds.rs`**: all 10 byte-slice sites refactored to `safe_slice(...)`
  (degree-sign tail check, label-forward / label-backward windows,
  header window for manufacturer + product, glass-transition window,
  print-speed window).
- **`sds.rs`**: all 5 byte-slice sites refactored to `safe_slice(...)`
  (section split, density window forward + backward, two Section-9
  range snippets).
- **`lib.rs`** — `run_command(...)` wrapper: every Tauri command
  (`import_pdf`, `import_from_urls`, `crawl_catalog`, `scan_corpus`)
  runs its body inside `std::panic::catch_unwind`. Any future panic
  surfaces as a structured `Error::Other("internal panic: …")`
  returned to the JS frontend instead of taking the worker thread
  down. The single safety net beneath the UTF-8 fix.
- **`lib.rs`** — persistent log file at
  `%LOCALAPPDATA%\Custom Filament Profile Creator\app.log` (Windows),
  `~/Library/Application Support/Custom Filament Profile Creator/app.log`
  (macOS), `~/.local/share/Custom Filament Profile Creator/app.log`
  (Linux). `env_logger` targets this file when running as a packaged
  app; a custom panic hook also appends panic info + location, so
  the next time something goes wrong the user has a file to send
  back. The release build no longer relies on stderr that nobody
  reads.

If you installed 0.1.7 and saw the window-closes-on-click symptom,
0.1.8 fixes it in place — the bundle identifier is unchanged
(`fork.leanspectrum.profile-creator`) so the new MSI / `.deb` / `.rpm`
upgrades over 0.1.7.

## [0.1.4–0.1.7] — IPC + serde plumbing (no functional changes)

Four rapid-fire bugfix releases to make the v0.1.2 build actually
usable end-to-end:

- **0.1.4** — visible boot-error trap so a JS syntax error in the
  loader script no longer paints a blank app window. Surfaced the
  v0.1.2-era scope collision below.
- **0.1.5** — fix `const t` from `main.js` colliding with `function t`
  from `i18n.js` at global scope (`Identifier 't' has already been
  declared`). Renamed the i18n helper to `tr` to free up the symbol.
  The drop zone, the three tabs and the language picker all start
  responding again.
- **0.1.6** — `invoke('import_pdf', { pdf_path })` reverted to the
  snake-case argument name expected by the Rust struct literal,
  removing the `missing field pdf_path` error.
- **0.1.7** — belt-and-suspenders: added
  `#[serde(rename_all = "camelCase")]` to `ImportRequest` /
  `BatchImportRequest` so both `pdf_path` and `pdfPath` deserialize
  correctly. The JS side is back to camelCase to keep the wire
  format consistent with Tauri's documented convention.

These four iterations shared a single underlying cause (a single
unaccounted-for runtime error blanked the entire frontend) and are
collapsed here for readability. The 0.1.8 entry above is where the
*data* path was made correct.

## [0.1.3] — CSP fix: drop zone + tabs were silently broken in 0.1.2

The 0.1.2 release installed correctly on Windows (MSI valid, app
launched, window painted) but neither the file drop zone nor the
three navigation tabs responded to user input. Root cause: the
Content-Security-Policy `script-src 'self'` blocked Tauri 2's
runtime injection of the `window.__TAURI__` global. The very first
line of `frontend/src/main.js` then threw
`TypeError: Cannot read properties of undefined (reading 'core')`,
which halted the entire script before any event listener
(tabs, drop zone, language picker, run button) could attach.

Fixes:
- **tauri.conf.json `security.csp`** rewritten to the Tauri 2
  recommended baseline:
  `default-src 'self' ipc: http://ipc.localhost; img-src 'self' data: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost`
  — adds `'unsafe-inline'` to `script-src`, opens `ipc:` +
  `http://ipc.localhost` for the IPC bridge, opens
  `asset: http://asset.localhost` for in-app asset URLs.
- **frontend/src/main.js** now resolves the invoke handle defensively
  via `resolveInvoke()` that probes both `window.__TAURI__.core.invoke`
  (Tauri 2 stable) and `window.__TAURI__.invoke` (legacy fallback),
  and logs a console error with diagnostic info if neither is found.
  The tabs and i18n picker work even when invoke is null — useful
  for triaging future runtime issues.
- **windows[0]**: `devtools: true` (so users can open F12 / right-click
  Inspect to debug) and `dragDropEnabled: true` (explicit, since Tauri
  2 stable changed the default).

If you installed v0.1.2 and saw the dead drop zone, uninstall it (the
app identifier changed across the rename so the v0.1.3 MSI installs
side-by-side, but the v0.1.2 entry in Add/Remove Programs is the same
ID `fork.leanspectrum.profile-creator` so the new MSI upgrades it
in-place).

## [0.1.2] — rename: Custom Filament Profile Creator

What changed:
- Product name **LeanSpectrum SDS Importer → Custom Filament Profile
  Creator** (FR display: *Créateur de profils filament adaptés*) — the
  new name reflects what the user actually gets: a Snapmaker_Orca
  filament profile *tailored* to the specific vendor filament, not just
  a generic SDS importer.
- Cargo package `leanspectrum-sds-importer → custom-filament-profile-creator`
- Tauri identifier `fork.leanspectrum.sds-importer → fork.leanspectrum.profile-creator`
- Tauri lib name `leanspectrum_sds_importer_lib → custom_filament_profile_creator_lib`
- npm package name + node-side workflow renamed to match
- Window title + bundle product name updated
- CI workflow trigger accepts both `profile-creator-v*` (new canonical)
  and `sds-importer-v*` (legacy)

Functional behaviour: unchanged from 0.1.1. Same SDS / TDS parser, same
output profile schema, same OCR fallback, same vendor catalog crawler.

The 0.1.0 and 0.1.1 release tags never published due to the icon bug
chain (fixed in 0.1.1's `bundle.icon` array). 0.1.2 is the first
publicly downloadable build.

## [0.1.1] — Windows bundle fix

The 0.1.0 release tag's Windows binary failed to bundle because
`tauri.conf.json` only listed `icons/icon.png` in `bundle.icon`,
which Tauri's bundler can't use as the Windows shortcut/EXE icon
(Windows requires `.ico` format). The `icons/icon.ico` file was
already present in the repo but not referenced.

0.1.1 lists all available icon variants (`icon.png`, `icon.ico`,
`32x32.png`, `128x128.png`, `128x128@2x.png`) in `bundle.icon` so
the Tauri bundler picks the right format per platform: `.ico` for
Windows MSI and NSIS bundles, `.png` for Linux `.deb` / `.rpm` /
`.AppImage`, the size variants for `.AppImage` icon-resolution
matching.

No other changes from 0.1.0. The Cargo source code is unchanged.

## [0.1.0] — first downloadable release

### What it does

Drop a filament Safety Data Sheet or Technical Data Sheet PDF; get a
Snapmaker_Orca filament profile written into your user-profile folder.
The next time you open Snapmaker_Orca, the new filament appears in
the Filament list — import it via the regular `Import` menu if you
prefer not to drop it directly in `~/.../Snapmaker_Orca/user/<id>/filament/`.

### Three input modes

- **Single PDF** — drag-and-drop or click to pick a PDF, optional
  online TDS lookup if the SDS only carries chemistry data.
- **Vendor catalog** — paste a "certificates" / "downloads" page URL
  (e.g. eryone3d.com/pages/filament-files,
  sunlu.com/products/.../downloads-section, atome3d.com/pages/...).
  The app fetches the page, lists every SDS / TDS PDF it can identify
  with `SDS` / `TDS` / `Unknown` badges and a polymer-family chip,
  and lets you batch-import any subset in one click.
- **Local database** — browse PDFs already on disk (defaults to
  `~/Downloads/filament-corpus/`, configurable). Files are grouped
  by brand sub-folder; click any to import.

### Parser coverage

Validated against 1600+ real vendor PDFs across 50+ brands collected
from the catalogue and direct manufacturer downloads. Specific
regression tests in `tds.rs` and `sds.rs` cover:

- Eryone (TDS, tabular layout with whitespace padding)
- eSun (SDS + TDS, French and English variants)
- ROSA3D (MSDS, Polish-style supplier line, reverse-column TDS)
- SUNLU (SDS with TDS data embedded in section 9, official TDS with
  values above labels)
- 10 polymer families detected by CAS number + name patterns:
  PLA, PETG, ABS, ASA, PC, TPU, PA6, PA12, HIPS, PP.

### Generated profile contents

The output JSON inherits from the closest Snapmaker U1-tuned
profile (`Snapmaker PLA SnapSpeed @U1`, `Generic ABS @U1`, …) so
you keep the parent profile's cooling / retract / pressure-advance
settings, and overrides only what the importer extracted or
backfilled:

- Nozzle temperature, range low / high, initial-layer temperature
- Bed temperature (start + initial layer)
- Filament density, vendor, filament type
- **Maximum volumetric speed** — extracted when present, otherwise
  filled from a conservative per-polymer default (PLA 12, PETG 9,
  ABS 9, PC 7, PA 8, TPU 4 mm³/s on a 0.4 mm nozzle)
- **Scarf-joint seam settings** tuned per polymer family
  (`seam_slope_min_length`, `seam_slope_steps`,
  `scarf_angle_threshold`, `scarf_joint_speed`,
  `scarf_joint_flow_ratio`, `seam_position`) — produces nearly
  invisible Z-seams on PLA / PETG / ABS / ASA / PC / PA / PP / HIPS
  with sensible defaults; disabled on TPU (rubber doesn't ramp
  cleanly).

Backfilled fields are tracked in `_leanspectrum_metadata.estimated_fields`
so the UI badges them as `Needs review` when the user should
sanity-check before printing critical parts.

### Bilingual UI

English and French built-in, switchable from the top bar; the
selection persists in `localStorage`. First launch follows the OS
locale.

### Optional OCR

Scanned (image-only) PDFs need OCR. To keep the default binary
slim and the CI cross-platform, OCR is gated behind the `ocr`
Cargo feature and is **not** in the prebuilt binaries. To enable
it, install system Tesseract and Leptonica then rebuild:

```bash
cargo tauri build --features ocr
```

### Platforms

| OS      | Artifact                              |
|---------|---------------------------------------|
| Linux   | `.AppImage` + `.deb` + `.rpm`         |
| macOS   | `.dmg` (separate arm64 and Intel)     |
| Windows | `.msi` (recommended) + `.exe` (NSIS)  |

### Known limitations

- Some vendor TDS layouts that pdftotext extracts column-first (eSun
  ePLA-HF in particular) are only partially parsed — nozzle temperature
  is picked up via a polymer-aware backward scan, but bed temperature
  and print speed in those layouts are lost. Workaround: drop the SDS
  for the same product instead, then the importer fills bed temperature
  from the polymer default.
- The local-database browser only walks one nesting level deep
  (`<corpus>/<brand>/*.pdf` and `<corpus>/<brand>/<product>/*.pdf`).
  Deeper trees are ignored.
- Process profile recommendation (which printer / nozzle preset to pair
  the filament with) returns `None` for now — to land in v0.2.

### License

Proprietary — strictly personal & private use only (see LICENSE.md). The parent
slicer (OptimusOrca / Snapmaker_Orca) stays AGPL-3.0-or-later. Built on:
- [Tauri 2](https://tauri.app/) — Apache-2.0 / MIT
- [pdf-extract](https://crates.io/crates/pdf-extract) — MIT / Apache-2.0
- [pdfium-render](https://crates.io/crates/pdfium-render) — Apache-2.0 / MIT
- [scraper](https://crates.io/crates/scraper) — ISC
- [tesseract-rs](https://crates.io/crates/tesseract) (optional) — Apache-2.0

ISO 11014-1 / GHS standards are public references; no proprietary
schema or profile content is reproduced.
