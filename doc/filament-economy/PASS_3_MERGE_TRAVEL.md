# Pass 3 — Merge travel and retract around swaps

> Eliminate redundant retract / un-retract sequences and merge consecutive
> travel-only moves around kept tool changes.

## Problem statement

Around every tool change, OrcaSlicer/Snapmaker_Orca emits a sequence
similar to:

```
G1 E-2.0 F1800        ; retract (end of previous extruder use)
G1 X.. Y.. F9000      ; travel to wipe tower
G1 E2.0 F1800         ; un-retract
... wipe tower extrusion ...
G1 E-2.0 F1800        ; retract before leaving the tower
G1 X.. Y.. F9000      ; travel to next print position
G1 E2.0 F1800         ; un-retract
```

When two consecutive swaps happen in quick succession — for instance with
FullSpectrum dithering where filament A is used briefly, then B briefly,
then A again — the inner `un-retract → retract` pair is functionally a
no-op:

```
... wipe A ...
G1 E-2 F1800          ; retract       <-- redundant
G1 X.. Y.. F9000      ; travel        <-- can be merged with the next one
G1 E2 F1800           ; un-retract    <-- redundant
... wipe B (immediately) ...
```

The two retracts and the two un-retracts cancel out. Removing them saves:

- 4 instructions per swap pair
- ~50 ms per swap pair (retract/un-retract speed is ~1800 mm/min)
- A small amount of filament motion that contributes to ooze/wear

## Approach

Walk the kept tool-change events (those not removed by Pass 1). For each
pair of consecutive swaps `(swap_i, swap_{i+1})`, if the distance and
time between the END of swap_i's purge and the START of swap_{i+1}'s
purge are small (configurable thresholds), collapse the retract /
un-retract bracket.

## Algorithm

```
Inputs: ordered list of kept ToolChange events with refined block_end
        (last G-code line of the previous swap's epilogue) and
        block_start (first G-code line of the next swap's prologue).

For each adjacent (a, b):
    gap_lines  = b.block_start - a.block_end
    gap_dist_mm = sum of XY distance of motion lines in the gap
    gap_time_s  = sum of feedrate-derived time of those lines

    if gap_lines <= MAX_GAP_LINES (default 8)
       and gap_dist_mm <= MAX_GAP_DIST (default 5 mm)
       and gap_time_s <= MAX_GAP_TIME (default 0.5 s):

        - Find a's epilogue retract (last G1 E<negative>)
          and b's prologue un-retract (first G1 E<positive> after the gap).
        - If they cancel within EPS_E (default 0.01 mm), comment them out.
        - Find a's epilogue un-retract and b's prologue retract — these
          form the inner symmetric pair. Same cancellation test, same
          treatment.
        - Merge the two travel moves into one diagonal:
              old: G1 X1 Y1 F   ;  G1 X2 Y2 F'
              new: G1 X2 Y2 F'' where F'' = min(F, F')
          but only if no `WIPE` or `AVOID_CROSSING` marker is in between.
```

## Edge cases

| Case                                    | Handling                                                                  |
|-----------------------------------------|---------------------------------------------------------------------------|
| Gap contains custom user G-code         | Skip — never collapse across user-injected gcode                          |
| Gap contains a `M104`/`M109` (set temp) | Skip — temperature changes have side effects                              |
| Gap contains `;TYPE:` change            | Skip — the gap is functional, leave alone                                 |
| Retract/un-retract use different E      | Skip — they are not cancelable                                            |
| Firmware retraction mode (`G10`/`G11`)  | Treat the pair `G10 ... G11` as a single retract/un-retract unit          |
| Z-hop included in retract               | Z-hop motion must be preserved if it crosses the gap; skip merge          |

## Risks

- **Heuristic mis-match** — collapsing a retract that turns out to be
  protecting a delicate motion. Mitigation: very conservative thresholds
  by default; the feature ships disabled (`filament_economy_merge_travel
  = false` in PrintConfig).
- **Cumulative E desync** — in absolute extrusion mode (`M82`), removing
  retracts changes the cumulative E. Implementation must re-emit corrected
  cumulative values after the cancellation, or refuse to run in M82 mode.
- **Time estimation** — the GCodeProcessor's print-time estimate is
  rebuilt from the G-code; rewriting may invalidate it. Mitigation: re-run
  the GCodeProcessor after Pass 3, or stub a small fix-up.

## Config

Already declared in `PrintConfig.cpp`:

- `filament_economy_merge_travel` (bool, default `false`)

To add in a follow-up commit:

- `filament_economy_merge_travel_max_gap_mm` (float, default 5)
- `filament_economy_merge_travel_max_gap_lines` (int, default 8)
- `filament_economy_merge_travel_max_gap_s` (float, default 0.5)

## Implementation order

Pass 3 is the most fragile of the three. Recommended sequencing:

1. Land Pass 1 (refined) + Pass 2 + tests.
2. Build a fixtures library with real U1 G-code captures.
3. Implement Pass 3 with extensive unit tests on those fixtures.
4. Keep the feature flag default `false` for at least one release cycle.

## Pseudocode skeleton

```cpp
struct GapInfo {
    size_t start_line;
    size_t end_line;
    double dist_mm;
    double time_s;
    bool   has_custom_gcode;
    bool   has_temp_change;
};

GapInfo measure_gap(const Parsed &p, size_t from_line, size_t to_line);

size_t pass_merge_travel(Parsed &p, Stats &stats, const Settings &s)
{
    size_t merged = 0;
    for (size_t i = 0; i + 1 < p.tool_changes.size(); ++i) {
        const ToolChange &a = p.tool_changes[i];
        const ToolChange &b = p.tool_changes[i + 1];
        GapInfo g = measure_gap(p, a.block_end, b.block_start);

        if (g.dist_mm > 5.0 || g.time_s > 0.5 || g.has_custom_gcode ||
            g.has_temp_change || (g.end_line - g.start_line) > 8)
            continue;

        if (collapse_retract_pair(p, a.block_end, b.block_start) > 0)
            ++merged;
        if (merge_two_travels(p, a.block_end, b.block_start) > 0)
            ++merged;
    }
    stats.lines_removed += merged;
    return merged;
}
```
