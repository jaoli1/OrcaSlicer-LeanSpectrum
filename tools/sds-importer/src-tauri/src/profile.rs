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

/// Choose the closest stock Snapmaker_Orca filament profile to inherit from.
/// We prefer the U1-tuned Snapmaker variant when one exists for the polymer
/// family (it carries U1 cooling / retract / pressure-advance tunings), and
/// fall back to the universal "Generic" profile otherwise.
fn inherit_stub_for(polymer: Polymer) -> &'static str {
    match polymer {
        Polymer::Pla       => "Snapmaker PLA SnapSpeed @U1",
        Polymer::Petg      => "Snapmaker PETG HF @U1",
        Polymer::Abs       => "Generic ABS @U1",
        Polymer::Asa       => "Generic ASA @U1",
        Polymer::Pc        => "Generic PC @U1",
        Polymer::Tpu       => "Generic TPU @U1",
        Polymer::NylonPa6  => "Generic PA @U1",
        Polymer::NylonPa12 => "Generic PA @U1",
        Polymer::Hips      => "Generic HIPS @U1",
        Polymer::Pp        => "Generic PP @U1",
        Polymer::Other     => "Generic PLA @U1",
    }
}

pub fn build_and_save(
    ef: &ExtractedFilament,
    log: &mut Vec<String>,
) -> Result<(Option<PathBuf>, Option<RecommendedProcess>)> {
    let polymer = ef.polymer.ok_or_else(|| Error::Profile(
        "Could not identify the polymer family — nothing to save.".into()
    ))?;

    let product_name = ef.product_name.as_deref().unwrap_or("Imported filament");
    let manufacturer = ef.manufacturer.as_deref().unwrap_or("Unknown");
    let display      = format!("{} — {} ({})", polymer.as_str(), product_name, manufacturer);

    let to_string_array = |v: Option<f64>| match v {
        Some(x) if x.is_finite() => json!([format!("{:.2}", x)]),
        _ => json!([""]),
    };

    // The three settings the user singled out as having the biggest impact
    // on print success: extrusion temperature, bed temperature, and the
    // maximum volumetric speed. For each we use the extracted value if any,
    // otherwise the polymer family default. We track which fields were
    // backfilled in `estimated_fields` so the UI can flag them for review.
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

    // Scarf-joint seam settings tuned per polymer (cf. polymer::default_scarf_settings).
    let scarf = polymer.default_scarf_settings();
    let scarf_value = build_scarf_value(&scarf);

    let profile = json!({
        "name":     display,
        "from":     "User",
        "type":     "filament",
        "inherits": inherit_stub_for(polymer),
        "filament_type":      [polymer.as_str()],
        "filament_vendor":    [manufacturer],

        // Highest-impact fields, set explicitly even when inheriting.
        "nozzle_temperature":             to_string_array(ef.nozzle_temp_recommended_c),
        "nozzle_temperature_range_low":   to_string_array(ef.nozzle_temp_min_c),
        "nozzle_temperature_range_high":  to_string_array(ef.nozzle_temp_max_c),
        "nozzle_temperature_initial_layer": to_string_array(ef.nozzle_temp_recommended_c),
        "hot_plate_temp":                 to_string_array(ef.bed_temp_min_c),
        "hot_plate_temp_initial_layer":   to_string_array(ef.bed_temp_min_c),
        "filament_density":               to_string_array(ef.density_g_cm3),
        "filament_max_volumetric_speed":  to_string_array(max_flow),

        // Scarf-seam fields. Merged into the JSON so the user can drop the
        // profile into Snapmaker_Orca and immediately get hidden seams on
        // appropriate geometry. Per-polymer values (see polymer.rs).
        "seam_position":               [scarf_value["seam_position"].as_str().unwrap_or("back")],
        "seam_slope_type":             [if scarf.enable_scarf { "external" } else { "none" }],
        "seam_slope_conditional":      [if scarf.enable_scarf { "1" } else { "0" }],
        "seam_slope_min_length":       [format!("{:.1}",   scarf.scarf_length_mm)],
        "seam_slope_steps":            [scarf.scarf_steps.to_string()],
        "seam_slope_entire_loop":      ["0"],
        "seam_slope_inner_walls":      ["0"],
        "scarf_angle_threshold":       [format!("{}",      scarf.scarf_angle_deg)],
        "scarf_joint_speed":           [format!("{}%",     scarf.scarf_joint_speed_pct)],
        "scarf_joint_flow_ratio":      [format!("{}%",     scarf.scarf_flow_ratio_pct)],

        "_leanspectrum_metadata": {
            "source":           "SDS/TDS importer",
            "polymer":          polymer.as_str(),
            "extracted_at":     Utc::now().to_rfc3339(),
            "estimated_fields": estimated_fields,
            "needs_review":     ef.needs_review,
            "source_files":     ef.source_files,
            "revision_date":    ef.revision_date,
            "scarf_settings":   scarf_value,
            "inherit_target":   inherit_stub_for(polymer),
        }
    });

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
