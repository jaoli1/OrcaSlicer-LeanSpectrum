//! OrcaSlicer filament profile generator.
//!
//! Builds a minimal JSON profile from an [`ExtractedFilament`] and writes
//! it into the user's Snapmaker_Orca profile directory. Also scans the
//! installed process profiles and recommends the best match for the
//! detected polymer family.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{polymer::{Polymer, ScarfSettings}, Error, ExtractedFilament, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilamentProfile(pub Value);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedProcess {
    pub name:          String,
    pub layer_height:  Option<f64>,
    pub print_speed:   Option<f64>,
    pub priority:      String, // "speed" | "quality" | "balanced"
    pub path:          String,
}

pub fn merge(into: &mut ExtractedFilament, other: ExtractedFilament) {
    macro_rules! prefer_other {
        ($field:ident) => {
            if into.$field.is_none() { into.$field = other.$field.clone(); }
        };
    }
    prefer_other!(product_name);
    prefer_other!(manufacturer);
    prefer_other!(revision_date);
    prefer_other!(polymer);
    prefer_other!(density_g_cm3);
    prefer_other!(glass_transition_c);
    prefer_other!(melt_temp_min_c);
    prefer_other!(melt_temp_max_c);
    prefer_other!(decomposition_c);
    prefer_other!(nozzle_temp_min_c);
    prefer_other!(nozzle_temp_max_c);
    prefer_other!(nozzle_temp_recommended_c);
    prefer_other!(bed_temp_min_c);
    prefer_other!(bed_temp_max_c);
    prefer_other!(bed_temp_recommended_c);
    prefer_other!(print_speed_min_mm_s);
    prefer_other!(print_speed_max_mm_s);
    prefer_other!(print_speed_recommended_mm_s);
    prefer_other!(max_flow_mm3_s);
    prefer_other!(fan_enabled);
    into.estimated_fields.extend(other.estimated_fields);
    into.source_files.extend(other.source_files);
}

/// Fill missing nozzle/bed temperatures using polymer defaults + SDS data.
pub fn estimate_missing_temperatures(ef: &mut ExtractedFilament, log: &mut Vec<String>) {
    if let Some(polymer) = ef.polymer {
        if ef.nozzle_temp_min_c.is_none() || ef.nozzle_temp_max_c.is_none() {
            // Strategy A: build from melting range + decomposition.
            if let (Some(melt_max), Some(decomp)) = (ef.melt_temp_max_c, ef.decomposition_c) {
                let lo = melt_max + 10.0;
                let hi = (lo + 30.0).min(decomp - 20.0);
                if hi > lo {
                    ef.nozzle_temp_min_c = Some(lo);
                    ef.nozzle_temp_max_c = Some(hi);
                    ef.nozzle_temp_recommended_c = Some((lo + hi) / 2.0);
                    ef.estimated_fields.push("nozzle_temp_min_c".into());
                    ef.estimated_fields.push("nozzle_temp_max_c".into());
                    log.push(format!("Estimated nozzle range {lo:.0}–{hi:.0} °C from SDS melting + decomposition values."));
                }
            }
            // Strategy B: polymer family default.
            if ef.nozzle_temp_min_c.is_none() {
                if let Some((lo, hi)) = polymer.default_nozzle_range_c() {
                    ef.nozzle_temp_min_c = Some(lo);
                    ef.nozzle_temp_max_c = Some(hi);
                    ef.nozzle_temp_recommended_c = Some((lo + hi) / 2.0);
                    ef.estimated_fields.push("nozzle_temp_min_c".into());
                    ef.estimated_fields.push("nozzle_temp_max_c".into());
                    log.push(format!("Used default nozzle range for {} family.", polymer.as_str()));
                }
            }
        }
        if ef.bed_temp_min_c.is_none() {
            if let Some((lo, hi)) = polymer.default_bed_range_c() {
                ef.bed_temp_min_c = Some(lo);
                ef.bed_temp_max_c = Some(hi);
                ef.estimated_fields.push("bed_temp_min_c".into());
                ef.estimated_fields.push("bed_temp_max_c".into());
                log.push(format!("Used default bed temperature for {} family.", polymer.as_str()));
            }
        }
    }
    ef.needs_review |= !ef.estimated_fields.is_empty();
}

pub(crate) fn snapmaker_orca_user_dir() -> Option<PathBuf> {
    let base = if cfg!(target_os = "macos") {
        dirs::data_dir().map(|d| d.join("Snapmaker_Orca"))
    } else if cfg!(target_os = "windows") {
        dirs::data_dir().map(|d| d.join("Snapmaker_Orca"))
    } else {
        dirs::config_dir().map(|d| d.join("Snapmaker_Orca"))
    };
    let user_dir = base?.join("user");
    if !user_dir.is_dir() { return None; }

    // Pick the most recently modified user UUID directory.
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(&user_dir).ok()?.flatten() {
        let p = entry.path();
        if !p.is_dir() { continue; }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                match best {
                    Some((t, _)) if t >= modified => {}
                    _ => best = Some((modified, p)),
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Choose the stock Snapmaker_Orca filament profile to inherit from.
///
/// CRITICAL: every name returned here MUST be the exact `name` of a profile
/// that (a) actually ships in `resources/profiles/Snapmaker/filament/` AND
/// (b) is compatible with `Snapmaker U1 (0.4 nozzle)`. If `inherits` points
/// at a non-existent profile the slicer silently fails to resolve the parent
/// and the imported filament falls back to bare defaults — which is exactly
/// what happened up to v0.1.9, where 9 of these 11 names did not exist
/// (e.g. "Generic ABS @U1", "Snapmaker PETG HF @U1", "Generic PA @U1" …).
///
/// Verified against the shipped profile set:
///   - PLA / PETG / ABS / ASA / TPU have a U1-tuned "Snapmaker X @U1" leaf.
///   - PC and PA have no "@U1" leaf but a U1-compatible "Generic X" exists
///     (compatible_printers lists "Snapmaker U1 (0.4 nozzle)").
///   - HIPS and PP have no profile at all → fall back to the nearest
///     thermal sibling that IS U1-compatible (HIPS≈ABS, PP≈PETG). The
///     real temps are overridden from the data sheet anyway, so only the
///     U1 hardware tuning (cooling / retraction / pressure advance) is
///     inherited.
fn inherit_stub_for(polymer: Polymer) -> &'static str {
    match polymer {
        Polymer::Pla       => "Snapmaker PLA SnapSpeed @U1",
        Polymer::Petg      => "Snapmaker PETG @U1",
        Polymer::Abs       => "Snapmaker ABS @U1",
        Polymer::Asa       => "Snapmaker ASA @U1",
        Polymer::Tpu       => "Snapmaker TPU @U1",
        Polymer::Pc        => "Generic PC",          // no @U1 leaf; Generic PC is U1-compatible
        Polymer::NylonPa6  => "Generic PA",          // no @U1 leaf; Generic PA is U1-compatible
        Polymer::NylonPa12 => "Generic PA",
        Polymer::Hips      => "Snapmaker ABS @U1",    // no HIPS profile; HIPS prints like ABS
        Polymer::Pp        => "Snapmaker PETG @U1",   // no PP profile; nearest mid-temp U1 leaf
        Polymer::Other     => "Snapmaker PLA SnapSpeed @U1",
    }
}

/// Format a float without trailing zeros: 220.0 -> "220", 1.23 -> "1.23".
/// Stock Snapmaker profiles store temperatures / counts as integer strings
/// and ratios as short decimals; this matches both.
fn fmt_num(x: f64) -> String {
    if x.fract().abs() < 1e-9 { format!("{x:.0}") } else { format!("{x}") }
}

/// The single print speed (mm/s) to inject into the process companion.
/// Priority: the manufacturer's validated test-specimen speed (authoritative),
/// then the midpoint of the recommended range, then the range top. `None` only
/// when the sheet carries no speed at all. Print speed is a PROCESS-domain
/// setting, so it would otherwise be silently dropped — this is what wires the
/// TDS "Printing speed" value into a profile the slicer actually applies.
fn effective_print_speed(ef: &ExtractedFilament) -> Option<f64> {
    ef.print_speed_recommended_mm_s
        .or_else(|| match (ef.print_speed_min_mm_s, ef.print_speed_max_mm_s) {
            (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() => Some(((lo + hi) / 2.0).round()),
            _ => None,
        })
        .or(ef.print_speed_max_mm_s)
        .filter(|v| v.is_finite() && *v > 0.0)
}

/// The stock U1 process profile the scarf companion attaches its overrides
/// to. "0.20 Standard" is the balanced default; the user can re-base to a
/// finer / faster layer height afterwards and keep the scarf overrides.
const BASE_PROCESS_U1: &str = "0.20 Standard @Snapmaker U1 (0.4 nozzle)";

/// Config version stamped on every generated USER preset.
///
/// CRITICAL: Snapmaker_Orca's preset loader (`Preset.cpp` ~L1220,
/// `if (!version) continue;`) SILENTLY DROPS any user preset whose `version`
/// key is missing or not a parseable Semver — the preset never appears in the
/// slicer's dropdown, with no error. This was THE reason generated profiles
/// were invisible. The value must be a 4-part Semver string and should be
/// <= the running slicer's `SLIC3R_VERSION` to avoid a forward-compat
/// migration pass; it matches the fork's `version.inc` SLIC3R_VERSION.
const PRESET_VERSION: &str = "01.10.01.70";

/// The printer preset generated presets target. A user preset is filtered out
/// of the dropdown unless its `compatible_printers` name-matches the active
/// printer (an empty list also passes, but being explicit is safer). Must be
/// the exact preset NAME, not a display alias.
const U1_PRINTER: &str = "Snapmaker U1 (0.4 nozzle)";

/// Build the companion **process** profile. Every import now gets one — it
/// carries the four fork-specific PROCESS-domain feature groups the slicer
/// applies on top of the stock U1 process:
///   1. scarf-joint seams           (per-polymer; `seam_*` / `scarf_*`)
///   2. the manufacturer print speed (`*_wall_speed` / `*_infill_speed`)
///   3. LeanSpectrum filament economy (`filament_economy_*`)
///   4. color-mixing readiness        (`mixed_filament_region_collapse`)
///
/// All of these are PROCESS-domain keys — a *filament* profile silently ignores
/// them (the v0.1.10 lesson). Verified in the fork's C++:
///   - `filament_economy_*` and `mixed_filament_*` are members of `PrintConfig`
///     (the process aggregate; `mixed_filament_*` are gated on
///     `Preset::TYPE_PRINT` in the GUI), and
///   - `FilamentEconomy::Settings::from_config()` reads them from
///     `full_print_config()`, which folds in the active process preset — so a
///     value set here genuinely reaches the post-processor.
///
/// Value formats follow the process-profile convention: plain string scalars
/// (NOT the filament profile's string-arrays), bools as "0"/"1".
///
/// Filament economy is enabled to match the fork's own C++ defaults — it
/// benefits every print (curvature-aware E scaling on single-color, purge
/// shrinking + no-op tool-change removal on FullSpectrum multi-color). For color
/// mixing only the safe `region_collapse` optimisation is set; the experimental
/// gradient / dithering / pointillism / bias modes are left at their off
/// defaults so single-color prints are unaffected (the user opts into those from
/// Process ▸ Others). `merge_travel` is left off (experimental) too.
/// Material-adaptive bed-adhesion / anti-warp settings:
/// (brim width mm, draft-shield enabled, first-layer speed mm/s).
/// High-warp families (ABS/ASA/PC/PA/HIPS) get a wide brim + draft shield + slow
/// first layer; PLA/PETG stay light; PLA gets only a small brim. Conservative
/// researched defaults — see data/RESEARCH_supports_adhesion.md — user-tunable.
fn anti_warp_for(p: Polymer) -> (f64, bool, f64) {
    use Polymer::*;
    match p {
        Abs | Asa | Pc | NylonPa6 | NylonPa12 | Hips => (8.0, true, 20.0),
        Petg | Pp => (5.0, false, 25.0),
        Tpu => (5.0, false, 20.0),
        Pla => (3.0, false, 25.0),
        Other => (4.0, false, 25.0),
    }
}

fn build_process_json(
    product_display: &str,
    scarf: &ScarfSettings,
    print_speed: Option<f64>,
    polymer: Polymer,
) -> (String, Value) {
    let speed = print_speed.filter(|x| x.is_finite() && *x > 0.0);
    let (brim_w, draft_shield, first_layer_speed) = anti_warp_for(polymer);
    // Name reflects the headline feature: scarf seams when enabled, else
    // "Tuned" (still carries filament economy + color-mixing readiness + speed).
    let kind = if scarf.enable_scarf { "Scarf" } else { "Tuned" };
    let name = format!("{product_display} {kind} @U1 (0.4 nozzle)");

    let mut v = json!({
        "name":     name,
        "version":  PRESET_VERSION,
        "from":     "User",
        "is_custom_defined": "1",
        "type":     "process",
        "inherits": BASE_PROCESS_U1,
        "compatible_printers": [U1_PRINTER],
        // --- LeanSpectrum filament economy. Matches the fork's C++ defaults,
        //     emitted explicitly so the saving is guaranteed regardless of what
        //     the inherited base process sets. ---
        "filament_economy_enable":            "1",
        "filament_economy_remove_noop_swaps": "1",
        "filament_economy_shrink_purge":      "1",
        "filament_economy_shrink_purge_pct":  "30",
        "filament_economy_curvature_lh":      "1",
        "filament_economy_force_m83":         "1",
        // --- Color-mixing readiness: the safe region-collapse optimisation
        //     only; experimental modes stay off. ---
        "mixed_filament_region_collapse":     "1",
        // --- supports tuned to grip the plate but peel cleanly off the MODEL
        //     (geometric; only take effect when the slice generates supports) ---
        "support_top_z_distance":       "0.2",
        "support_bottom_z_distance":    "0.2",
        "support_interface_spacing":    "0.5",
        "support_interface_pattern":    "rectilinear",
        "support_interface_top_layers": "2",
        "support_object_xy_distance":   "0.35",
        "_leanspectrum_metadata": {
            "source":       "SDS/TDS importer — process companion",
            "base_process": BASE_PROCESS_U1,
            "fork_features": {
                "scarf_seams":       scarf.enable_scarf,
                "print_speed_mm_s":  speed,
                "filament_economy":  true,
                "color_mixing_note": "region-collapse on; experimental gradient/dither/pointillism/bias left off — opt in via Process > Others",
            }
        }
    });
    {
        let obj = v.as_object_mut().expect("process root is a JSON object");

        if scarf.enable_scarf {
            // coFloat ratio: 100% -> 1.0, 95% -> 0.95.
            let flow_ratio = scarf.scarf_flow_ratio_pct as f64 / 100.0;
            obj.insert("seam_slope_type".into(),        json!("external"));
            obj.insert("seam_slope_conditional".into(), json!("1"));
            obj.insert("scarf_angle_threshold".into(),  json!(scarf.scarf_angle_deg.to_string()));
            obj.insert("scarf_joint_speed".into(),      json!(format!("{}%", scarf.scarf_joint_speed_pct)));
            obj.insert("scarf_joint_flow_ratio".into(), json!(fmt_num(flow_ratio)));
            obj.insert("seam_slope_min_length".into(),  json!(fmt_num(scarf.scarf_length_mm)));
            obj.insert("seam_slope_steps".into(),       json!(scarf.scarf_steps.to_string()));
            obj.insert("seam_position".into(),          json!(scarf.seam_position.clone()));
        }

        if let Some(s) = speed {
            // The manufacturer's recommended print speed applies to the main
            // print moves. coFloat plain-string scalars in a process profile.
            for key in ["outer_wall_speed", "inner_wall_speed",
                        "sparse_infill_speed", "internal_solid_infill_speed"] {
                obj.insert(key.to_string(), json!(fmt_num(s)));
            }
        }

        // --- material-adaptive bed adhesion / anti-warp ---
        // brim (outer, fused to the part for max grip) sized by warp tendency;
        // draft shield + slow first layer for the high-warp families; a slow
        // first layer also helps the supports anchor to the plate.
        obj.insert("brim_type".into(),       json!("outer_only"));
        obj.insert("brim_width".into(),      json!(fmt_num(brim_w)));
        obj.insert("brim_object_gap".into(), json!("0"));
        obj.insert("draft_shield".into(),    json!(if draft_shield { "enabled" } else { "disabled" }));
        obj.insert("initial_layer_speed".into(), json!(fmt_num(first_layer_speed)));
    }
    (name, v)
}

/// Build the filament profile JSON document (pure; no disk I/O). Split out
/// from `build_and_save` so the schema can be unit-tested without touching
/// the user's Snapmaker_Orca directory.
fn build_profile_json(
    ef: &ExtractedFilament,
    polymer: Polymer,
    log: &mut Vec<String>,
) -> Value {
    let product_name = ef.product_name.as_deref().unwrap_or("Imported filament");
    let manufacturer = ef.manufacturer.as_deref().unwrap_or("Unknown");
    let display      = format!("{} — {} ({})", polymer.as_str(), product_name, manufacturer);

    // The settings with the biggest impact on print success: extrusion
    // temperature, bed temperature, maximum volumetric speed. Each uses the
    // extracted value if any, otherwise the polymer-family default. Backfilled
    // fields are tracked in `estimated_fields` so the UI can flag them.
    let mut estimated_fields = ef.estimated_fields.clone();

    let max_flow = ef.max_flow_mm3_s
        .filter(|v| v.is_finite() && *v > 0.0)
        .or_else(|| {
            let v = polymer.default_max_flow_mm3_s();
            if v.is_some() {
                estimated_fields.push("filament_max_volumetric_speed".into());
                log.push(format!(
                    "Used polymer-family default max volumetric speed ({} mm^3/s) — vendor sheet did not provide one.",
                    v.unwrap_or(0.0)
                ));
            }
            v
        });

    // Scarf-joint seam settings tuned per polymer. NOTE: `seam_*` / `scarf_*`
    // are PROCESS-domain keys — Snapmaker_Orca ignores them inside a
    // *filament* profile (no stock filament profile carries them, every
    // process profile does). Through v0.1.9 we emitted ~10 such keys at the
    // top level where they did nothing. We now keep them only in
    // `_leanspectrum_metadata` for reference and a future process-profile
    // companion, and do not write dead top-level keys.
    let scarf = polymer.default_scarf_settings();
    let scarf_value = build_scarf_value(&scarf);

    // Mandatory skeleton. Data-driven keys are inserted below ONLY when we
    // actually have a value, so we never overwrite an inherited parent value
    // with an empty string (the old `to_string_array(None) -> [""]` bug could
    // blank the parent's nozzle/bed temperature).
    let mut profile = json!({
        "name":     display,
        "version":  PRESET_VERSION,
        "from":     "User",
        "is_custom_defined": "1",
        "type":     "filament",
        "inherits": inherit_stub_for(polymer),
        "compatible_printers": [U1_PRINTER],
        "filament_type":   [polymer.as_str()],
        "filament_vendor": [manufacturer],
        "_leanspectrum_metadata": {
            "source":           "SDS/TDS importer",
            "polymer":          polymer.as_str(),
            "extracted_at":     Utc::now().to_rfc3339(),
            "estimated_fields": estimated_fields,
            "needs_review":     ef.needs_review,
            "source_files":     ef.source_files,
            "revision_date":    ef.revision_date,
            "scarf_settings":   scarf_value,
            "scarf_note":       "seam_*/scarf_* are process-domain keys; apply them in a process profile, not here",
            "inherit_target":   inherit_stub_for(polymer),
        }
    });

    {
        let obj = profile.as_object_mut().expect("profile root is a JSON object");

        // Scalar filament keys: emit each only when present + finite.
        let scalars: [(&str, Option<f64>); 7] = [
            ("nozzle_temperature",               ef.nozzle_temp_recommended_c),
            ("nozzle_temperature_initial_layer", ef.nozzle_temp_recommended_c),
            ("nozzle_temperature_range_low",     ef.nozzle_temp_min_c),
            ("nozzle_temperature_range_high",    ef.nozzle_temp_max_c),
            ("filament_density",                 ef.density_g_cm3),
            ("temperature_vitrification",        ef.glass_transition_c),
            ("filament_max_volumetric_speed",    max_flow),
        ];
        for (key, v) in scalars {
            if let Some(x) = v.filter(|x| x.is_finite()) {
                obj.insert(key.to_string(), json!([fmt_num(x)]));
            }
        }

        // Bed temperature applies to whichever build plate is selected, so
        // set ALL four plate-type keys + their initial-layer variants. The
        // U1's default plate is textured PEI; setting only `hot_plate_temp`
        // (as we did through v0.1.9) left the textured plate at the parent
        // default and the user's bed temperature silently never changed.
        //
        // Value priority: the manufacturer's validated test-specimen bed temp
        // (authoritative — e.g. ERYONE's "base plate 60 °C"), then the midpoint
        // of the recommended range (rounded — 55-70 → 63), then the range low
        // end. Through v0.1.12 we always used the low end, which under-set the
        // bed vs. what the vendor actually printed at.
        let bed_value = ef.bed_temp_recommended_c
            .or_else(|| match (ef.bed_temp_min_c, ef.bed_temp_max_c) {
                (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() => Some(((lo + hi) / 2.0).round()),
                _ => None,
            })
            .or(ef.bed_temp_min_c)
            .filter(|x| x.is_finite());
        if let Some(bed) = bed_value {
            for key in [
                "hot_plate_temp",      "hot_plate_temp_initial_layer",
                "cool_plate_temp",     "cool_plate_temp_initial_layer",
                "eng_plate_temp",      "eng_plate_temp_initial_layer",
                "textured_plate_temp", "textured_plate_temp_initial_layer",
            ] {
                obj.insert(key.to_string(), json!([fmt_num(bed)]));
            }
        }
    }

    profile
}

pub fn build_and_save(
    ef: &ExtractedFilament,
    log: &mut Vec<String>,
) -> Result<(Option<PathBuf>, Option<RecommendedProcess>)> {
    let polymer = ef.polymer.ok_or_else(|| Error::Profile(
        "Could not identify the polymer family — nothing to save.".into()
    ))?;

    let scarf = polymer.default_scarf_settings();
    let product_disp = ef.product_name.clone()
        .unwrap_or_else(|| polymer.as_str().to_string());

    let profile = build_profile_json(ef, polymer, log);
    let display = profile["name"].as_str().unwrap_or("Imported filament").to_string();

    let user_dir = snapmaker_orca_user_dir();
    if user_dir.is_none() {
        log.push("Snapmaker_Orca user directory not found; saving the profile(s) next to the source PDF instead.".into());
    }
    // Resolve <user>/<sub> (filament | process), or the source PDF's folder
    // as a fallback when the slicer's user directory can't be located.
    let resolve_dir = |sub: &str| -> Result<PathBuf> {
        match user_dir.as_ref() {
            Some(p) => {
                let d = p.join(sub);
                fs::create_dir_all(&d)?;
                Ok(d)
            }
            None => Ok(ef.source_files
                .first()
                .map(PathBuf::from)
                .and_then(|p| p.parent().map(|x| x.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))),
        }
    };

    // 1) Filament profile.
    let filament_dir = resolve_dir("filament")?;
    let out_path = write_unique_json(&filament_dir, &display, &profile)?;
    log.push(format!("Saved filament profile to {}", out_path.display()));

    // 2) Companion process profile carrying the fork's PROCESS-domain features:
    //    scarf-joint seams, the manufacturer print speed, LeanSpectrum filament
    //    economy, and color-mixing readiness. These only take effect in a
    //    process profile — not in the filament profile. Every import gets one
    //    (filament economy + color mixing always apply); we inherit the standard
    //    U1 process, override only those keys, and surface it as the recommended
    //    process so the UI can point the user straight at it.
    let print_speed = effective_print_speed(ef);
    let (proc_name, proc_json) = build_process_json(&product_disp, &scarf, print_speed, polymer);
    let process_dir = resolve_dir("process")?;
    let proc_path = write_unique_json(&process_dir, &proc_name, &proc_json)?;
    let mut feats: Vec<String> = Vec::new();
    if scarf.enable_scarf { feats.push("scarf seams".into()); }
    if let Some(s) = print_speed { feats.push(format!("{s:.0} mm/s print speed")); }
    feats.push("filament economy".into());
    feats.push("color-mixing ready".into());
    log.push(format!(
        "Saved fork-tuned process profile ({}) to {}. Pick it in the Process dropdown to apply them.",
        feats.join(" + "),
        proc_path.display()
    ));
    let recommended = Some(RecommendedProcess {
        name:         proc_name,
        layer_height: Some(0.20),
        print_speed,
        priority:     "balanced".into(),
        path:         proc_path.display().to_string(),
    });

    Ok((Some(out_path), recommended))
}

/// Write `value` to `dir/<sanitized name>.json`, appending " (N)" if a file
/// with that name already exists. Shared by the filament and process writers.
pub(crate) fn write_unique_json(dir: &Path, display: &str, value: &Value) -> Result<PathBuf> {
    let mut path = dir.join(format!("{}.json", sanitize(display)));
    let mut counter = 1;
    while path.exists() {
        path = dir.join(format!("{} ({}).json", sanitize(display), counter));
        counter += 1;
    }
    fs::write(&path, serde_json::to_string_pretty(value).unwrap())
        .map_err(|e| Error::Profile(e.to_string()))?;
    Ok(path)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect()
}

fn build_scarf_value(s: &ScarfSettings) -> Value {
    json!({
        "enable_scarf":         s.enable_scarf,
        "scarf_joint_speed_pct": s.scarf_joint_speed_pct,
        "scarf_length_mm":      s.scarf_length_mm,
        "scarf_steps":          s.scarf_steps,
        "scarf_flow_ratio_pct": s.scarf_flow_ratio_pct,
        "scarf_angle_deg":      s.scarf_angle_deg,
        "seam_position":        s.seam_position,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    /// The 11 inherit targets MUST be exactly the names that ship in
    /// resources/profiles/Snapmaker/filament/ and are U1-compatible. This
    /// pins the v0.1.10 fix that replaced 9 non-existent parents (which made
    /// inheritance silently fail) with verified ones. If a future profile
    /// rename breaks one of these, this test is the early warning.
    #[test]
    fn inherit_targets_are_the_verified_set() {
        assert_eq!(inherit_stub_for(Polymer::Pla),       "Snapmaker PLA SnapSpeed @U1");
        assert_eq!(inherit_stub_for(Polymer::Petg),      "Snapmaker PETG @U1");
        assert_eq!(inherit_stub_for(Polymer::Abs),       "Snapmaker ABS @U1");
        assert_eq!(inherit_stub_for(Polymer::Asa),       "Snapmaker ASA @U1");
        assert_eq!(inherit_stub_for(Polymer::Tpu),       "Snapmaker TPU @U1");
        assert_eq!(inherit_stub_for(Polymer::Pc),        "Generic PC");
        assert_eq!(inherit_stub_for(Polymer::NylonPa6),  "Generic PA");
        assert_eq!(inherit_stub_for(Polymer::NylonPa12), "Generic PA");
        assert_eq!(inherit_stub_for(Polymer::Hips),      "Snapmaker ABS @U1");
        assert_eq!(inherit_stub_for(Polymer::Pp),        "Snapmaker PETG @U1");
        assert_eq!(inherit_stub_for(Polymer::Other),     "Snapmaker PLA SnapSpeed @U1");
        // None may be empty — an empty `inherits` resolves to nothing.
        for p in [Polymer::Pla, Polymer::Petg, Polymer::Abs, Polymer::Asa,
                  Polymer::Tpu, Polymer::Pc, Polymer::NylonPa6, Polymer::NylonPa12,
                  Polymer::Hips, Polymer::Pp, Polymer::Other] {
            assert!(!inherit_stub_for(p).is_empty());
        }
    }

    fn eryone_pla() -> ExtractedFilament {
        // Represents the REAL ERYONE PLA+ extraction AFTER the v0.1.13 fix: the
        // test-specimen note ("printing temperature=210, printing speed=80,
        // base plate 60") supplies the authoritative recommended values that
        // override the parameter-table midpoints (which would be nozzle 205,
        // bed-low 55, speed-mid 65).
        ExtractedFilament {
            product_name:        Some("Eryone PLA+".into()),
            manufacturer:        Some("Shenzhen Eryone Technology Co,.Ltd".into()),
            polymer:             Some(Polymer::Pla),
            density_g_cm3:       Some(1.23),
            glass_transition_c:  Some(54.0),
            nozzle_temp_min_c:   Some(190.0),
            nozzle_temp_max_c:   Some(220.0),
            nozzle_temp_recommended_c: Some(210.0),
            bed_temp_min_c:      Some(55.0),
            bed_temp_max_c:      Some(70.0),
            bed_temp_recommended_c: Some(60.0),
            print_speed_min_mm_s: Some(30.0),
            print_speed_max_mm_s: Some(100.0),
            print_speed_recommended_mm_s: Some(80.0),
            ..Default::default()
        }
    }

    /// The generated PLA profile must inherit from the real U1 PLA parent,
    /// carry the data-sheet values as filament keys, set ALL plate-type
    /// temperatures (not just hot_plate), emit the glass-transition key, and
    /// must NOT carry the dead process-domain seam/scarf keys at the top
    /// level.
    #[test]
    fn pla_profile_has_correct_schema() {
        let mut log = Vec::new();
        let v = build_profile_json(&eryone_pla(), Polymer::Pla, &mut log);

        assert_eq!(v["inherits"], "Snapmaker PLA SnapSpeed @U1");
        assert_eq!(v["type"],     "filament");
        // Registration keys WITHOUT which the slicer silently drops the
        // preset (Preset.cpp ~1220 requires a parseable version) or hides it
        // (compatible_printers must name-match the active printer).
        assert_eq!(v["version"], "01.10.01.70");
        assert_eq!(v["is_custom_defined"], "1");
        assert_eq!(v["compatible_printers"][0], "Snapmaker U1 (0.4 nozzle)");
        assert_eq!(v["filament_type"][0], "PLA");
        assert_eq!(v["filament_vendor"][0], "Shenzhen Eryone Technology Co,.Ltd");

        // Temperatures as integer strings (no trailing ".00"). Nozzle is the
        // AUTHORITATIVE specimen-note value (210), NOT the 190-220 midpoint 205.
        assert_eq!(v["nozzle_temperature"][0],            "210");
        assert_eq!(v["nozzle_temperature_range_low"][0],  "190");
        assert_eq!(v["nozzle_temperature_range_high"][0], "220");
        assert_eq!(v["filament_density"][0],              "1.23");
        assert_eq!(v["temperature_vitrification"][0],     "54");

        // ALL four plate types + initial-layer variants carry the bed temp —
        // the specimen-note "base plate 60", NOT the 55 range low.
        for key in ["hot_plate_temp", "hot_plate_temp_initial_layer",
                    "cool_plate_temp", "cool_plate_temp_initial_layer",
                    "eng_plate_temp", "eng_plate_temp_initial_layer",
                    "textured_plate_temp", "textured_plate_temp_initial_layer"] {
            assert_eq!(v[key][0], "60", "plate key {key} should be the bed temp");
        }

        // Dead process-domain keys MUST NOT appear at the top level anymore.
        let obj = v.as_object().unwrap();
        for dead in ["seam_position", "seam_slope_type", "seam_slope_conditional",
                     "scarf_angle_threshold", "scarf_joint_speed", "scarf_joint_flow_ratio"] {
            assert!(!obj.contains_key(dead), "{dead} must not be a top-level filament key");
        }
        // …but the scarf reference is preserved in metadata.
        assert!(v["_leanspectrum_metadata"]["scarf_settings"].is_object());
    }

    /// The companion process profile must carry the scarf keys in the
    /// process-profile format (plain string scalars, bool as "0"/"1", flow
    /// ratio as a float not a percent), inherit the standard U1 process, and
    /// be typed as a process.
    #[test]
    fn scarf_companion_process_has_correct_schema() {
        let scarf = Polymer::Pla.default_scarf_settings();
        assert!(scarf.enable_scarf, "PLA scarf should be enabled by default");
        // 80 mm/s is the ERYONE specimen-note print speed.
        let (name, v) = build_process_json("Eryone PLA+", &scarf, Some(80.0), Polymer::Pla);

        assert_eq!(name, "Eryone PLA+ Scarf @U1 (0.4 nozzle)");
        assert_eq!(v["type"], "process");
        assert_eq!(v["inherits"], "0.20 Standard @Snapmaker U1 (0.4 nozzle)");
        // Same registration keys required for the process preset to load+show.
        assert_eq!(v["version"], "01.10.01.70");
        assert_eq!(v["is_custom_defined"], "1");
        assert_eq!(v["compatible_printers"][0], "Snapmaker U1 (0.4 nozzle)");
        // Process scalars are plain strings, not arrays.
        assert_eq!(v["seam_slope_type"], "external");
        assert_eq!(v["seam_slope_conditional"], "1");
        assert_eq!(v["scarf_angle_threshold"], "155");
        assert_eq!(v["scarf_joint_speed"], "50%");
        // coFloat ratio, NOT "100%".
        assert_eq!(v["scarf_joint_flow_ratio"], "1");
        assert_eq!(v["seam_slope_min_length"], "20");
        assert_eq!(v["seam_slope_steps"], "10");
        // None of these should be string-arrays (that's the filament format).
        assert!(v["seam_slope_type"].is_string());
        // The TDS print speed is injected into the main print moves — this is
        // the v0.1.13 gap fix (print speed is process-domain, was dropped).
        assert_eq!(v["outer_wall_speed"], "80");
        assert_eq!(v["inner_wall_speed"], "80");
        assert_eq!(v["sparse_infill_speed"], "80");
        assert_eq!(v["internal_solid_infill_speed"], "80");
        assert!(v["outer_wall_speed"].is_string());
        // v0.1.14: the fork's filament economy is enabled (process-domain keys
        // read by FilamentEconomy::Settings::from_config(full_print_config())).
        assert_eq!(v["filament_economy_enable"], "1");
        assert_eq!(v["filament_economy_remove_noop_swaps"], "1");
        assert_eq!(v["filament_economy_shrink_purge"], "1");
        assert_eq!(v["filament_economy_shrink_purge_pct"], "30");
        assert_eq!(v["filament_economy_curvature_lh"], "1");
        assert_eq!(v["filament_economy_force_m83"], "1");
        // …and the safe color-mixing optimisation, but NOT the experimental modes.
        assert_eq!(v["mixed_filament_region_collapse"], "1");
        // v0.1.19: supports tuned for clean model release + material-adaptive
        // anti-warp (PLA → light 3 mm brim, no draft shield).
        assert_eq!(v["support_interface_pattern"], "rectilinear");
        assert_eq!(v["support_top_z_distance"], "0.2");
        assert_eq!(v["brim_type"], "outer_only");
        assert_eq!(v["brim_width"], "3");
        assert_eq!(v["draft_shield"], "disabled");
        let obj = v.as_object().unwrap();
        for off in ["mixed_filament_gradient_mode", "mixed_filament_advanced_dithering",
                    "mixed_filament_component_bias_enabled", "filament_economy_merge_travel"] {
            assert!(!obj.contains_key(off), "{off} must be left at its (off) default");
        }
    }

    /// Every import now gets a process companion — even a polymer with scarf
    /// disabled (TPU) and no print speed still gets a "Tuned" companion carrying
    /// filament economy + color-mixing readiness. The headline feature (the
    /// whole project) must never be silently dropped.
    #[test]
    fn process_companion_always_generated_with_fork_features() {
        let scarf = Polymer::Tpu.default_scarf_settings();
        assert!(!scarf.enable_scarf, "TPU scarf should be disabled");
        let (name, v) = build_process_json("Generic TPU", &scarf, None, Polymer::Tpu);
        assert_eq!(name, "Generic TPU Tuned @U1 (0.4 nozzle)");
        assert_eq!(v["type"], "process");
        // Filament economy + color mixing present regardless of scarf/speed.
        assert_eq!(v["filament_economy_enable"], "1");
        assert_eq!(v["mixed_filament_region_collapse"], "1");
        let obj = v.as_object().unwrap();
        // No scarf keys (TPU) and no speed keys (no speed given).
        for absent in ["seam_slope_type", "scarf_joint_speed", "outer_wall_speed"] {
            assert!(!obj.contains_key(absent), "{absent} should be absent here");
        }
    }

    /// Scarf-disabled polymer WITH a print speed → "Tuned" companion carrying
    /// the speed (plus economy/mixing).
    #[test]
    fn speed_only_companion_emitted_when_scarf_disabled() {
        let scarf = Polymer::Tpu.default_scarf_settings();
        assert!(!scarf.enable_scarf, "TPU scarf should be disabled");
        let (name, v) = build_process_json("Generic TPU", &scarf, Some(30.0), Polymer::Tpu);
        assert_eq!(name, "Generic TPU Tuned @U1 (0.4 nozzle)");
        assert_eq!(v["type"], "process");
        assert_eq!(v["outer_wall_speed"], "30");
        assert_eq!(v["filament_economy_enable"], "1");
        // Scarf keys must be absent when scarf is disabled.
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("seam_slope_type"));
        assert!(!obj.contains_key("scarf_joint_speed"));
    }

    /// effective_print_speed: specimen-note recommendation wins; else range
    /// midpoint (rounded); else range top; else None.
    #[test]
    fn effective_print_speed_priority() {
        let mut ef = ExtractedFilament { polymer: Some(Polymer::Pla), ..Default::default() };
        assert_eq!(effective_print_speed(&ef), None);
        ef.print_speed_min_mm_s = Some(30.0);
        ef.print_speed_max_mm_s = Some(100.0);
        assert_eq!(effective_print_speed(&ef), Some(65.0)); // midpoint
        ef.print_speed_recommended_mm_s = Some(80.0);
        assert_eq!(effective_print_speed(&ef), Some(80.0)); // recommendation wins
    }

    /// When a value is missing we must NOT emit the key at all (emitting
    /// `[""]` would overwrite the inherited parent value with an empty
    /// string — the bug fixed in v0.1.10).
    #[test]
    fn missing_values_do_not_emit_empty_keys() {
        let ef = ExtractedFilament {
            polymer: Some(Polymer::Pla),
            // No nozzle / bed / density / glass-transition.
            ..Default::default()
        };
        let mut log = Vec::new();
        let v = build_profile_json(&ef, Polymer::Pla, &mut log);
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("nozzle_temperature"));
        assert!(!obj.contains_key("nozzle_temperature_range_low"));
        assert!(!obj.contains_key("filament_density"));
        assert!(!obj.contains_key("temperature_vitrification"));
        assert!(!obj.contains_key("hot_plate_temp"));
        assert!(!obj.contains_key("textured_plate_temp"));
        // Max volumetric speed still comes from the polymer default.
        assert_eq!(v["filament_max_volumetric_speed"][0], "12");
    }
}
