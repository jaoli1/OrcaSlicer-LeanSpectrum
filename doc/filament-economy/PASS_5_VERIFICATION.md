# Pass 5 — Physical correctness verification

> Enforces M83 (relative extrusion), retract preservation, mass
> conservation and flow-limit compliance. Required to run before any
> other pass that rewrites E values. Direct port of Al-Juboori 2026
> §3.7 / §3.10.

## What it does

Pass 5 is a **gate**, not an optimiser. It runs first, asserts a set of
invariants, and either:

- Marks the file as safe for downstream passes (Pass 1, 2, 3, 4, 6, 7).
- Or refuses to optimise and falls back to the unmodified input.

It also performs one rewrite if needed: **converting M82 absolute mode
into M83 relative mode**. The paper shows this is essential — without
relative E, the cumulative arithmetic errors from per-segment scaling
explode after a few thousand lines.

## Invariants asserted

### I1 — Relative extrusion

The file must use M83. If `M82` appears without a later `M83`, Pass 5
walks the file and converts every absolute E value into incremental
form:

```
incremental_E_i = absolute_E_i - absolute_E_{i-1}
```

with reset on every G92 E0 statement. The resulting file has `M83` at
the top and every `G1 E<x>` carries a delta.

### I2 — Retract preservation

Pass 5 records the set of retract events (negative E, no XY motion or
F-only motion) and the cumulative retract volume from the input. After
all downstream passes, this set must be identical line-by-line. Any
discrepancy halts output.

```
retract_count_in == retract_count_out
retract_volume_in == retract_volume_out  (mm³)
```

### I3 — Mass conservation within feature

For each (feature, layer) bucket, the new total `Σ E` must equal
`(1 - mean_reduction) * old_total` to within 0.5 %. Larger drift
suggests a parser bug.

### I4 — Bounded local modification

For every individual segment, `|E_new - E_old| / |E_old| ≤ cap`
where `cap` is the feature-specific bound (see Pass 4 doc).
Violations are logged and the offending segment is reverted to the
original E.

### I5 — Volumetric flow compliance

For each segment, compute:

```
Q_i = layer_height_i * extrusion_width_i * feedrate_i
```

and reject the rewrite if `max(Q_i) > 0.9 * Q_max(material)`. Defaults:

| Material | Q_max (mm³/s) |
|----------|---------------|
| PLA      | 15            |
| PETG     | 11            |
| ABS      | 12            |
| Nylon    | 12            |
| TPU      | 5             |

These match the paper's Nylon limit (12 mm³/s) and Snapmaker U1's
published spec for PLA SnapSpeed (15 mm³/s).

## Public API

```cpp
namespace FilamentEconomy {

struct VerificationReport
{
    bool   ok                = false;       // I1..I5 all passed
    bool   converted_to_m83  = false;       // I1 rewrite happened
    size_t retract_count_in  = 0;           // I2 source
    double retract_volume_in_mm3 = 0;       // I2 source
    double max_flow_mm3s     = 0;           // I5 observed peak
    std::vector<std::string> failures;      // human-readable
};

VerificationReport verify_and_normalise(const std::string &gcode_path,
                                        const Settings    &settings);

// Re-checks I2 / I3 after downstream passes finish.
bool verify_post_optimisation(const std::string &gcode_path,
                              const VerificationReport &pre,
                              const Settings &settings,
                              Stats &stats);

}
```

## Algorithm

```
1. Open the file, walk line-by-line, build a Parsed snapshot.

2. Detect E mode:
   - If first non-comment 'M82' appears before any 'M83' → absolute mode.
   - Otherwise relative.

3. If absolute mode AND settings.economy_force_m83 → rewrite to M83.
   Insert 'M83' at the top, walk every G1 E<x> converting to incremental.
   Handle G92 E0 resets correctly.

4. Walk again, classifying each line: retract, unretract, deposition,
   travel, layer-change, custom. Record per-class totals.

5. Compute Q_i for each deposition segment using modal layer height and
   modal feedrate. Track max.

6. If max Q > 0.9 * Q_max(material) → I5 fail, return early.

7. Store retract count and cumulative volume in the report. Return.
```

After downstream passes run, the caller invokes `verify_post_optimisation`
which:

- Re-counts retracts.
- Re-computes cumulative retract volume.
- Re-computes per-feature `Σ E`.
- Compares each against the pre-report's numbers within tolerance.
- If any check fails, the file is reverted to the original (whose
  contents Pass 5 saved to a `.le-backup` sibling file).

## Settings to add

| Key                                  | Type  | Default | Purpose                       |
|--------------------------------------|-------|---------|-------------------------------|
| `filament_economy_force_m83`         | bool  | true    | Convert M82→M83 if needed     |
| `filament_economy_max_flow_factor`   | float | 0.9     | I5 safety factor              |
| `filament_economy_mass_tolerance_pct`| float | 1.0     | I3 max drift                  |
| `filament_economy_max_local_pct`     | int   | 30      | I4 cap (also used by Pass 4)  |

## Tests

- M82 file → converted to M83, E sum unchanged.
- File with retracts → counts match before/after no-op pass.
- Synthetic file with Q > limit → rejected, modified=false.
- Properly-relative file → no rewrite, ok=true.
