# Porting plan — Dynamic Infill Purging (DIP)

Status: **mostly moot — feature is already mainline OrcaSlicer**.
Discovered 2026-05-27 by grepping `repo/src/libslic3r/PrintConfig.cpp`.
The "community fork doing DIP" the initial survey pointed at appears
to have been hyping existing upstream functionality.

## What's already in OrcaSlicer

Three config keys plus the underlying scheduler:

| Key | Default | Behavior |
|---|---|---|
| `flush_into_infill` | **false** | Purge after filament change is routed into the active object's sparse infill instead of the wipe tower. Reduces both waste and print time. Caveat: visible through transparent walls. |
| `flush_into_support` | **true** | Purge routed into support material. Generally safe. |
| `flush_into_objects` | **false** | Purge routed into the object itself (any region, not just infill). Aggressive — mixes colors at the surface. |

All three are gated behind the prime tower being enabled.

## What LeanSpectrum changes

Just the defaults. AutoProfile now sets `flush_into_infill = true`
and `flush_into_support = true` as part of every intent, because on
the Snapmaker U1's multi-color workflow we always want to reduce
wipe-tower volume — that's the entire point of the LeanSpectrum
filament-economy story.

Users who print with transparent walls can flip `flush_into_infill`
back off from the Multi-material settings tab — the value is
preserved in the project preset.

## Sequencing with Pass 2 (shrink purge)

Pass 2 sees only the residual purge that wasn't routed into infill.
The shrink ratio in Pass 2 already accounts for actual purge volume
left after upstream routing, so:

  Order: OrcaSlicer's flush-into-infill scheduler  →  Pass 2 shrink
  Effect: stacks multiplicatively — DIP reroutes 60-80% of purge
          into infill, then Pass 2 shrinks the residual another
          0-30% depending on idle time.

Pass 5's mass-conservation invariant already absorbs both rewrites
because each one updates `stats.extrusion_saved_mm` independently.

## What's NOT here

True opacity-aware DIP — modulating purge density per filament's
optical opacity so transparent walls don't show the mixed-color
infill underneath — is NOT in mainline. PrusaSlicer doesn't have
it either. Would be a real feature port, but the user benefit is
narrow (only transparent / translucent filaments) and the
implementation is heavy. Defer until users ask.

---

## Original (pre-discovery) port sketch — superseded by the above

The section below was written before the audit revealed mainline
already has DIP. Kept for the algorithmic intuition only; the
actual integration plan is the AutoProfile change documented above.

## What it does

Instead of routing every filament-change purge into a dedicated wipe
tower (the current behaviour, even with our Pass 2 shrink), DIP
reroutes the purge into the *internal sparse infill* of the active
object. The transition plastic is deposited where it will be hidden
inside the print, eliminating most or all of the wipe tower.

For an 8-color print on a Snapmaker U1 with FullSpectrum, the wipe
tower can account for 25–40 % of total filament use. DIP can drop
that to near-zero on parts with enough internal volume to absorb the
purge.

## Algorithm sketch

For each tool-change in the G-code:

1. Estimate purge volume required for this transition
   (this is what Pass 2 already computes via the per-pair flush
   matrix from project_config).
2. Look ahead in the toolpath for the next sparse-infill segment
   that uses the incoming filament.
3. If the segment has enough volume capacity, *prepend* the purge to
   it (extending the segment to absorb the purge volume).
4. If not enough capacity, fall back to:
   a. The remaining capacity goes into infill.
   b. The rest goes into a (shrunk) wipe tower.
5. Track purge accounting so Pass 5's mass-conservation invariant
   still balances after the rewrite.

## Settings (~6 expected)

| Key | Default | Purpose |
|---|---|---|
| `dynamic_infill_purging_enable` | false | Master toggle |
| `dip_max_capacity_pct` | 80 | Max fraction of an infill segment that can be replaced by purge |
| `dip_visible_infill_protection` | true | Skip infill segments adjacent to a visible surface |
| `dip_min_segment_volume_mm3` | 50 | Skip too-small segments |
| `dip_opacity_safety_factor` | 0.7 | Conservative reduction for pigment-mismatch risk |
| `dip_fallback_to_wipe_tower` | true | Allow partial purge to wipe tower if infill is full |

## Conflicts with existing architecture

1. **Pass 2 (shrink purge)** computes a reduction factor based on
   per-extruder idle time. With DIP, the residual wipe-tower volume
   is what Pass 2 sees. Sequence:
   - DIP rewrite first (relocates purge into infill, leaves residual
     in wipe tower).
   - Pass 2 second (shrinks the residual).
   - Pass 5 verifies mass conservation across both rewrites.
2. **Pass 4 (curvature E scaling)** must not scale the purged infill
   segments — they need full flow. Add a "do-not-scale" marker on
   DIP-purged segments and have Pass 4 honour it via a new flag on
   the parsed line.
3. **FullSpectrum cadence**: when the active layer is virtual (mix
   of A + B), DIP must purge into the *physical* infill that the
   layer actually deposits. This is the trickiest interaction —
   needs careful prototyping.

## Sources to study

Searches to run before starting:
- `site:github.com OrcaSlicer "purge into infill"` — find community forks
- `site:github.com PrusaSlicer "purge into object"` — find the upstream
- `site:reddit.com/r/3Dprinting "dynamic infill purging"` — user
  reports + edge cases

DIP is a flagship PrusaSlicer feature; the OrcaSlicer community ports
vary in quality. Pick the cleanest one as the porting source.

## Porting steps

1. Identify source fork with the cleanest DIP implementation.
2. Diff against its upstream-merge tag.
3. Identify scope: typically `GCode.cpp`, `Print.cpp`, possibly a
   new `DynamicInfillPurger.cpp`.
4. Add config keys (off by default).
5. Add the rewrite step in the slicer pipeline *before* the
   filament-economy passes (so Pass 2 sees the residual wipe).
6. Add a "purged" marker on parsed lines that Pass 4 reads.
7. Test cases:
   - 2-color print, sufficient infill volume → wipe tower removed
     entirely.
   - 4-color print with thin walls + low infill → fallback to wipe
     tower for some purges.
   - FullSpectrum dithered print → DIP routes through physical
     infill only on the correct half-layer.

## Estimated effort

| Phase | Effort |
|---|---|
| Source fork survey + selection | 1 day |
| Diff identification | half day |
| Config keys + sequencing in pipeline | 1 day |
| DIP algorithm port | 5 to 8 days |
| Pass 2 / 4 / 5 interaction adapter | 2 to 3 days |
| FullSpectrum interaction tests | 2 days |
| **Total** | **2 to 3 weeks** |

## Decision flag

Defer DIP until after Wave Overhangs lands and a real U1 print has
validated the existing pipeline end-to-end. DIP is the heaviest of the
three top-ranked features and the most likely to interact badly with
our economy passes if rushed.
