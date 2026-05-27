# Pass 2 — Shrink purge volumes

> Reduce the extrusion inside wipe-tower purges when the previous use of
> the target extruder is recent enough that a full-volume purge is not
> needed.

## Problem statement

When the slicer schedules a tool change, it adds a *purge* — a fixed
volume of plastic extruded to clear oozed/contaminated filament from the
nozzle. Snapmaker_Orca emits this as a wipe-tower segment marked with
specific comments:

```
;TYPE:Wipe tower
; CP TOOLCHANGE START
... motion + extrusion ...
; CP TOOLCHANGE END
```

The volume is computed from a static `filament_minimal_purge_on_wipe_tower`
config + a pairwise matrix `flush_volumes_matrix[from_idx][to_idx]`. It
assumes worst case: the target nozzle has cooled and oozed for the longest
expected idle.

In practice, when FullSpectrum alternates filaments layer by layer, each
extruder is only idle for the time it takes to print one or two layers.
The full purge is then significantly oversized.

## Approach

For every kept tool change (after Pass 1), look at the **idle time** of the
target extruder since its last use, then reduce the purge by a factor `s`:

```
s = clamp(idle_time / saturation_time, 0, 1)
purge_new = purge_old * (minimum_ratio + s * (1 - minimum_ratio))
```

Where:

- `idle_time` = wall-clock seconds since the target extruder last extruded.
- `saturation_time` = configurable, default 600 s; beyond that we assume
  the nozzle is fully cool and the purge should not be reduced at all.
- `minimum_ratio` = `1 - filament_economy_shrink_purge_pct / 100`.
  Default `shrink_purge_pct = 30` → minimum_ratio = 0.7.

So a freshly-used extruder gets `purge * 0.7` (30 % reduction); a long-idle
extruder gets `purge * 1.0` (no reduction).

## Algorithm

```
1. Walk the G-code once to compute per-extruder "last extrusion time"
   markers, using an estimated wall-clock built from feedrate + distance.

2. Walk the G-code a second time, looking for "; CP TOOLCHANGE START"
   markers. For each:
     a. Identify the target tool from the next "T<n>" line.
     b. Look up that tool's idle_time at the current position.
     c. Compute s and the new purge ratio r = (minimum_ratio + s * ...).
     d. Walk forward until "; CP TOOLCHANGE END", multiplying every
        positive E<value> by r. Negative E values (retracts) are left
        untouched — they must match their un-retract to keep the
        extruder primed correctly.
     e. Record the saved length for Stats.

3. The X/Y/Z motion of the wipe tower is *not* changed. Only the E values
   are reduced. The head still traces the tower outline; it just lays
   down less plastic per millimetre of travel.
```

## Edge cases

| Case                                    | Handling                                            |
|-----------------------------------------|-----------------------------------------------------|
| First tool change of the print          | `idle_time = infinity` → no reduction               |
| Two swaps in same layer (advanced multi)| Each gets its own idle calculation independently    |
| Wipe tower spans multiple T<n> internally (Bambu-style hot-swap) | Treat each `TOOLCHANGE START..END` block independently |
| Pass 1 already removed the swap         | Skip — no purge to shrink                            |
| User disabled wipe tower entirely       | No `CP TOOLCHANGE START` markers → Pass 2 is a no-op |
| Relative extrusion mode (`M83`)         | E values are deltas, simple multiplication works    |
| Absolute extrusion mode (`M82`)         | E values are cumulative; must rewrite as offsets and re-emit cumulative values (more invasive) |

The absolute-extrusion case is the trickiest. Initial implementation only
supports relative mode; if absolute mode is detected (`M82` seen and no
`M83` later), Pass 2 logs a warning and skips.

## Risks

- **Quality regression** — too aggressive reduction → color bleed.
  Mitigation: cap `shrink_purge_pct` at 100 in config; default 30 is
  conservative.
- **Bias interaction** — when `mixed_filament_bias` is non-zero, one
  filament of the pair is recessed, changing the effective idle pattern.
  Pass 2 must use the *physical* extruder index, not the virtual mixed-
  filament slot.
- **Time estimation accuracy** — our wall-clock estimate is rough (no
  acceleration model). Errors here translate to noisy `s` values.
  Acceptable as long as `minimum_ratio` floors the reduction.

## Config

Already declared in `PrintConfig.cpp`:

- `filament_economy_shrink_purge` (bool, default true)
- `filament_economy_shrink_purge_pct` (int 0..100, default 30)

To add in a follow-up commit:

- `filament_economy_shrink_purge_saturation_s` (int, default 600)

## Pseudocode

```cpp
struct ExtruderState {
    double last_extrusion_time = -std::numeric_limits<double>::infinity();
};

double pass_shrink_purge(Parsed &p, Stats &stats, const Settings &s)
{
    const double saturation_s   = 600.0;
    const double min_ratio      = 1.0 - s.shrink_purge_pct / 100.0;

    std::array<ExtruderState, kMaxExtruders> ex;
    double clock_s = 0.0;
    int    active  = 0;

    // Phase 1: estimate per-extruder last-extrusion timestamps.
    for (auto &line : p.lines)
        update_clock_and_active(line, clock_s, active, ex);

    // Phase 2: rewrite purge segments.
    double total_saved_mm = 0.0;
    for (size_t i = 0; i < p.lines.size(); ++i) {
        if (p.lines[i].find("; CP TOOLCHANGE START") != std::string::npos) {
            int target_tool = find_next_T_target(p, i);
            double idle    = clock_at_line(p, i) - ex[target_tool].last_extrusion_time;
            double sat     = std::clamp(idle / saturation_s, 0.0, 1.0);
            double ratio   = min_ratio + sat * (1.0 - min_ratio);
            total_saved_mm += rewrite_purge_E_values(p, i, ratio);
            ++stats.purges_shrunk;
        }
    }
    stats.extrusion_saved_mm += total_saved_mm;
    return total_saved_mm;
}
```
