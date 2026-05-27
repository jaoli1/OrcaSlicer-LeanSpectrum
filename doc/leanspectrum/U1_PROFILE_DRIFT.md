# U1 profile drift report — task 22b

Cross-check between AutoProfile's hand-tuned intent table and the
stock `Snapmaker/process/*` + `Snapmaker/filament/*` + `Snapmaker/machine/*`
profile chain. Audit date: 2026-05-27.

## Method

For each of `max_volumetric_speed`, `retraction_length`,
`outer_wall_speed`, trace the inheritance:

```
Process: fdm_process_U1.json → fdm_process_U1_<height>.json → user pick
Filament: fdm_filament_pla.json → Snapmaker PLA @U1 base.json → user pick
Machine: fdm_U1.json → Snapmaker U1 (0.4 nozzle).json
```

Compare each effective stock value against `AutoProfile::overrides_for`
(intent table) and `refine_for` (per-polymer scaling).

## Results

### Retraction

| Source | Value | Notes |
|---|---|---|
| `fdm_U1.json` | 0.8 mm @ 30 mm/s | Parent U1 default |
| `Snapmaker U1 (0.4 nozzle).json` | 1.5 mm @ 30 mm/s | Final, overrides parent |
| AutoProfile PLA | 0.8 mm @ 40 mm/s | conservative |
| AutoProfile PETG | 1.5 mm @ 40 mm/s | stringy material |
| Snapmaker wiki envelope | 0.5..3 mm @ 30..70 mm/s | |

**Verdict**: 1.5 mm machine default is a polymer-agnostic midpoint
between PLA (0.8) and PETG (1.5). Speed 30 mm/s is at the conservative
end of the wiki envelope. AutoProfile activation correctly overrides
to a polymer-aware value via `filament_retraction_length`. No drift.

### Volumetric flow

| Source | Value | Notes |
|---|---|---|
| `fdm_filament_pla.json` | 14 mm³/s | Stock PLA default |
| AutoProfile Standard (PLA) | 22 mm³/s | intent × 1.0 PLA scale |
| AutoProfile Draft (PLA) | 28 mm³/s | maximum suggested |
| Snapmaker U1 hardware ceiling | 32 mm³/s | wiki |

**Drift detected (minor)**: stock PLA filament profile caps at 14 mm³/s,
which is more conservative than even AutoProfile HighQuality (15) and
half of AutoProfile Draft (28). Users who skip the Auto-Profile button
are stuck at 14 even on a Draft intent.

**Recommendation**: bump `fdm_filament_pla.json::filament_max_volumetric_speed`
to **20 mm³/s** so stock-Standard slicing matches AutoProfile Standard.
AutoProfile-driven overrides still apply for Draft/HighQuality.

Action: deferred — value change has user-visible consequences
(slightly faster default prints, potentially more under-extrusion on
weak hotends). Belongs in a versioned release-notes entry, not a hot
fix. Tracked as a follow-up to 22b.

### Outer wall speed

| Source | Value | Notes |
|---|---|---|
| `fdm_process_U1.json` | 120 mm/s | Stock process default |
| AutoProfile Draft | 80 mm/s | most aggressive intent |
| AutoProfile HighQuality | 40 mm/s | most conservative |

**Intentional gap, not drift**: stock process is "go fast unless told
otherwise"; AutoProfile is "curated conservative bundle". Both can be
correct depending on user posture. Documenting it here so it doesn't
look accidental.

### Purge volume

| Source | Value | Notes |
|---|---|---|
| `0.20 Standard @Snapmaker U1 (0.4 nozzle)` | `prime_volume: 45 mm³` | |
| Snapmaker wiki guidance | 40..60 mm³ per color swap | |
| Earlier `_old.json` snapshot | `prime_volume: 15 mm³` | deleted in same commit batch |

**No drift**. The current 45 mm³ sits in the wiki sweet spot. The
`_old.json` 15 mm³ was a stale snapshot from before the prime-tower
tuning campaign and is no longer reachable through `Snapmaker.json`.

## Out-of-scope

- Per-filament `nozzle_temperature` cross-check vs Snapmaker wiki
  filament library (Polymer-specific, would need 73-file pass)
- `cooling_overhang_threshold`, `min_layer_time` deltas vs AutoProfile
  (intent-level, not material-level — not a U1 reliability axis)
- IDEX-specific timing (`z_hop`, `standby_temperature_delta`,
  `preheat_time`) — covered under task 22d (start/end G-code audit)

## Conclusion

One minor drift (PLA filament `max_volumetric_speed` 14 vs AutoProfile
Standard 22) recommended for a follow-up tuning PR. No blocking
inconsistencies.
