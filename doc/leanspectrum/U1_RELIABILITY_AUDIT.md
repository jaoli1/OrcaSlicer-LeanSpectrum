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
