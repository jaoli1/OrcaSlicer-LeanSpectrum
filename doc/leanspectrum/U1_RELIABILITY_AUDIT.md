# U1 Reliability Audit — Task #22 decomposition

Snapshot 2026-05-27. Task #22 was a vague "fiabiliser SnapmakerOrca pour
U1". This audit replaces it with concrete sub-tasks that can be picked
off independently.

## Current state (already shipped on `feature/filament-economy`)

- 1 machine profile family: `Snapmaker U1.json`, `Snapmaker U1 (0.4
  nozzle).json`, `fdm_U1.json` — 3 machine JSONs total
- 23 process profiles (`*Snapmaker U1*.json` under
  `resources/profiles/Snapmaker/process/`)
- 73 filament profiles bound to U1 (Snapmaker-branded ABS / ASA / PA-CF
  / PETG-CF / PLA / Support / TPU, plus Polymaker / PolyLite /
  PolyTerra third-party)
- **AutoProfile** module hand-tuned against the official Snapmaker U1
  wiki: 32 mm³/s volumetric cap, 0.5-3 mm retract, 40-60 mm³ purge,
  per-extruder Z-hop, 230-250 °C PLA range
- **BambuConvert** + **FullSpectrumDither** wire Bambu palettes into
  the U1's 4-extruder bay with optional virtual mixing
- **5-pass FilamentEconomy** post-processor with rollback gate

## Identified gaps (decomposed below)

### G1. Upstream Snapmaker fixes not yet absorbed

`git log feature/filament-economy..upstream-snap/main` lists 25 commits
not in our branch. The bugfix-flavoured ones are the priority:

| Commit | Title | Priority | Notes |
|---|---|---|---|
| `1c2df8e498` | Bugfix : http port conflict + zero-density filament display | **High** | Touches network + filament UI — likely cherry-pickable |
| `948f46e6ba` | Fix wipe tower exceed limit | **High** | Wipe tower interacts with our Pass 2/3 |
| `99f25f4eed` | bug fix for soft | Medium | Vague title, needs commit-body read |
| `2b31abeada` | fix: model download URL encoding | Medium | LAN upload reliability |
| `94dea7e8a9` | update soft version | Low | Version bump only |
| `ee2b0d74c6` | Square wipe tower | Low | Aesthetic; verify no conflict with Pass 2 |
| `ac3dafe08a` | Feat:mix filament | Skip | Conflicts with FullSpectrum + BambuConvert mixing |
| `32e8ed7c7f` | Feature filament modify | Skip | Same conflict scope |

### G2. Profile coherence audit

- `0.20 Standard @Snapmaker U1 (0.4 nozzle)_old.json` still ships — the
  `_old` suffix suggests it was kept as a fallback. Decide: keep,
  rename, or delete.
- Cross-reference each process profile's `max_volumetric_speed`,
  `retract_length`, `purge_volume` against the AutoProfile intent
  tables to confirm no drift.
- Verify every filament with `@U1` suffix has a `default_filament_profile`
  that resolves under `Snapmaker U1.json`.

### G3. Start / end G-code sanity check

- `machine_start_gcode` for U1 should include MZV input-shaping
  preheat, accelerometer-trigger-safe homing, and the IDEX Z-offset
  query.
- `machine_end_gcode` should park to the IDEX wipe position, not the
  centre bed.
- Verify the prime-tower G-code matches the forum-validated
  `3-8 mm brim` recommendation.

### G4. Network layer (LAN upload) U1-specific paths

- `src/slic3r/Utils/PresetUpdater.cpp`, `Http.cpp`, `Obico.cpp`,
  `Process.cpp`, `SimplyPrint.cpp` all reference "Snapmaker" but no
  U1-specific code path was found in a 3-file grep. The U1 LAN upload
  likely falls through to a generic OctoPrint-style PUT.
- Check whether the `1c2df8e498` upstream HTTP port fix lands here.

### G5. Crash-class issues

No specific crash reports filed against this fork yet. Action: enable
Sentry / breadcrumb logging on the next release (release-only build
flag already wired in build_all.yml — `Upload Debug Symbols to Sentry`
job is currently `skipped` because no `SENTRY_AUTH_TOKEN` secret is
configured).

## Recommended sub-task split

| # | Task | Effort | Risk |
|---|---|---:|---|
| 22a | Cherry-pick upstream fixes `1c2df8e498` + `948f46e6ba` | 1 h | Low |
| 22b | Audit U1 process profile coherence vs AutoProfile intent | 2 h | Low |
| 22c | Decide `_old.json` fate (keep+rename or delete) | 15 min | None |
| 22d | Validate U1 start/end/prime-tower G-code blocks | 1 h | Low |
| 22e | Network layer — absorb URL-encoding + port fixes | 30 min | Low |
| 22f | Configure SENTRY_AUTH_TOKEN secret to enable crash uploads | 15 min | None |
| 22g | (Optional) Absorb `ee2b0d74c6` square wipe tower iff Pass 2 unaffected | 1 h | Medium |

Total: ~5-6 h focused work for 22a-22f. 22g is opportunistic.

## Out-of-scope (intentional)

- `ac3dafe08a` Feat:mix filament — would collide with our
  FullSpectrum + BambuConvert mixing
- `32e8ed7c7f` Feature filament modify — same reason
- Wave Overhangs Phase 3b/4/5 merge — covered by separate experiment
  branch PR
- Dynamic Infill Purging port — already tracked under task #58

## Next step

Mark task #22 itself as `decomposed/completed` after this audit lands;
create 22a..22f as separate trackable tasks; tackle them in order of
priority (22a, 22d, 22e first — all touch user-facing reliability).

---

# Audit execution log (2026-05-27)

## 22a — Cherry-pick upstream-snap bugfixes  ✅

Two surgical fixes landed:

- **948f46e6ba** Wipe tower byte-budget overrun. 3-line addition to
  `WipeTowerIntegration::post_process_wipe_tower_moves` (`else
  continue;` to skip no-op G1/G2/G3 moves). Commit `adff16600b`.
- **4b5fc8c3cd** (sub-commit of `2b31abeada`) URL with query
  parameters. `Downloader.cpp::filename_from_url` now uses libcurl's
  CURLU parser with a manual query-strip fallback. Commit
  `adff16600b`.

The third candidate, **1c2df8e498**, was deferred wholesale because of
246k lines of regenerated Flutter `main.dart.js`. The two
actually-substantive C++ deltas (HttpServer `/wcp_download/` route +
SSWCP `url` field in `sw_GetActiveFile`) were ported manually under
22e (commit `116eea607d`).

## 22b — Profile coherence  ✅

See `doc/leanspectrum/U1_PROFILE_DRIFT.md`. One minor drift: stock
`fdm_filament_pla.json::filament_max_volumetric_speed = 14` mm³/s vs
AutoProfile Standard = 22. Documented but **not changed** — value
shift belongs in versioned release notes, not a hot fix. Commit
`cb0e9e5076`.

## 22c — _old.json fate  ✅

Deleted `0.20 Standard @Snapmaker U1 (0.4 nozzle)_old.json`. Setting
ID `GP004` collided with the canonical file; `Snapmaker.json` only
ever referenced the canonical path so the orphan was unreachable
through the UI. Commit `cb0e9e5076`.

## 22d — Start / end / prime-tower G-code  ✅ (no changes)

Cross-checked `resources/profiles/Snapmaker/machine/Snapmaker U1
(0.4 nozzle).json` against the Snapmaker U1 multi-color printing
guide:

| Check | Stock value | Verdict |
|---|---|---|
| MZV input shaping preheat | Klipper firmware boot (not slicer-side) | ✓ correct |
| Accelerometer-safe homing | `G28 X Y` → cleaning → `G28 Z I140 J140` | ✓ correct |
| IDEX Z-offset query | `DETECT_BED_PLATE` + `BED_MESH_CALIBRATE PROBE_COUNT=11,11` | ✓ correct |
| Prime tower brim 3-8 mm | `prime_tower_brim_width: 5` (uniform across all 11 U1 process profiles) | ✓ correct |
| End G-code | ` PRINT_END\nTIMELAPSE_STOP` (Klipper macros handle parking/cooldown) | ✓ correct |

Note: the `fdm_U1.json` parent profile still has the old per-tool
inline purge G-code, but `Snapmaker U1 (0.4 nozzle).json` overrides
it. The actual G-code emitted to a real U1 uses the Klipper macro
form, which is correct.

No changes required.

## 22e — LAN upload URL encoding + port handling  ✅

Already absorbed by 22a (Downloader.cpp filename_from_url) and the
manual port of 1c2df8e498's C++ subset (commit `116eea607d`):

- `HttpServer::map_url_to_file_path` now decodes the `/wcp_download/`
  route with URL-safe base64
- `ResponseFile` gains a `m_native_path` flag so the base64-decoded
  raw bytes are opened directly
- `sw_GetActiveFile` emits the `url` field with the live port read
  from `m_page_http_server.get_port()`

## 22f — Sentry crash telemetry  ⏸ deferred

The `Upload Debug Symbols to Sentry` job in `build_all.yml` is wired
but skipped because `SENTRY_AUTH_TOKEN` is not configured as a repo
secret. Enabling it requires:

1. Creating a Sentry organization + project (web signup, ~5 min)
2. Generating an internal-integration auth token with `project:write`
   and `project:releases` scopes
3. Adding `SENTRY_AUTH_TOKEN` as a repo secret in
   `Settings → Secrets and variables → Actions`
4. Setting the Sentry DSN as a build-time `#define` in
   `src/sentry_wrapper/SentryWrapper.cpp` (currently hard-coded to
   an example value)

Deferred to a separate decision: do we want Sentry crash uploads at
v0.1.0 (paid SaaS, $0/month free tier for <5k events), or do we ship
crash-quiet first and add telemetry post-release? Tracked but not
blocking the upcoming PR.

## 22g — Square wipe tower  ⏸ deferred (opportunistic)

Pass 2 (shrink purge) already optimises the cone-shaped wipe tower
that ships today. Switching to square geometry is aesthetic and
doesn't gain measurable filament economy on a U1 (the IDEX bay is
fixed-pitch so the prime stroke distance doesn't change with tower
shape). Will revisit if a user reports an alignment regression on the
cone variant.

