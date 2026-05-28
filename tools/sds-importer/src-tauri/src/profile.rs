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
    prefer_other!(print_speed_min_mm_s);
    prefer_other!(print_speed_max_mm_s);
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

fn snapmaker_orca_user_dir() -> Option<PathBuf> {
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

    // Format a float without trailing zeros: 220.0 -> "220", 1.23 -> "1.23".
    // Stock Snapmaker filament profiles store temperatures as integer strings
    // and density as a short decimal string; this matches both.
    fn fmt_num(x: f64) -> String {
        if x.fract().abs() < 1e-9 { format!("{x:.0}") } else { format!("{x}") }
    }

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
        "from":     "User",
        "type":     "filament",
        "inherits": inherit_stub_for(polymer),
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
        if let Some(bed) = ef.bed_temp_min_c.filter(|x| x.is_finite()) {
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

    let profile = build_profile_json(ef, polymer, log);
    let display = profile["name"].as_str().unwrap_or("Imported filament").to_string();

    let user_dir = snapmaker_orca_user_dir();
    let filament_dir = match user_dir.as_ref() {
        Some(p) => {
            let d = p.join("filament");
            fs::create_dir_all(&d)?;
            d
        }
        None => {
            log.push("Snapmaker_Orca user directory not found; saving the profile next to the source PDF instead.".into());
            ef.source_files
                .first()
                .map(|p| PathBuf::from(p))
                .and_then(|p| p.parent().map(|x| x.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        }
    };

    let mut out_path = filament_dir.join(format!("{}.json", sanitize(&display)));
    let mut counter = 1;
    while out_path.exists() {
        out_path = filament_dir.join(format!("{} ({}).json", sanitize(&display), counter));
        counter += 1;
    }
    fs::write(&out_path, serde_json::to_string_pretty(&profile).unwrap())
        .map_err(|e| Error::Profile(e.to_string()))?;
    log.push(format!("Saved filament profile to {}", out_path.display()));

    let recommended = user_dir.as_ref().and_then(|p| recommend_process(p, polymer));
    Ok((Some(out_path), recommended))
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

fn recommend_process(_user_dir: &Path, _polymer: Polymer) -> Option<RecommendedProcess> {
    // Placeholder: real implementation walks the slicer's system/process/
    // directory, parses each .json, filters by the active printer + nozzle
    // size and the polymer's compatible_filaments field, then scores by
    // (print_speed, layer_height). For v0.1 we return None so the UI
    // shows "no recommendation yet" instead of a wrong default.
    None
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
        ExtractedFilament {
            product_name:        Some("Eryone PLA+".into()),
            manufacturer:        Some("Shenzhen Eryone Technology Co,.Ltd".into()),
            polymer:             Some(Polymer::Pla),
            density_g_cm3:       Some(1.23),
            glass_transition_c:  Some(54.0),
            nozzle_temp_min_c:   Some(190.0),
            nozzle_temp_max_c:   Some(220.0),
            nozzle_temp_recommended_c: Some(205.0),
            bed_temp_min_c:      Some(55.0),
            bed_temp_max_c:      Some(70.0),
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
        assert_eq!(v["filament_type"][0], "PLA");
        assert_eq!(v["filament_vendor"][0], "Shenzhen Eryone Technology Co,.Ltd");

        // Temperatures as integer strings (no trailing ".00").
        assert_eq!(v["nozzle_temperature"][0],            "205");
        assert_eq!(v["nozzle_temperature_range_low"][0],  "190");
        assert_eq!(v["nozzle_temperature_range_high"][0], "220");
        assert_eq!(v["filament_density"][0],              "1.23");
        assert_eq!(v["temperature_vitrification"][0],     "54");

        // ALL four plate types + initial-layer variants carry the bed temp.
        for key in ["hot_plate_temp", "hot_plate_temp_initial_layer",
                    "cool_plate_temp", "cool_plate_temp_initial_layer",
                    "eng_plate_temp", "eng_plate_temp_initial_layer",
                    "textured_plate_temp", "textured_plate_temp_initial_layer"] {
            assert_eq!(v[key][0], "55", "plate key {key} should be the bed temp");
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
