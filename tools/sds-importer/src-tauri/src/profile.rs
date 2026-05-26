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

use crate::{polymer::Polymer, Error, ExtractedFilament, Result};

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

fn inherit_stub_for(polymer: Polymer) -> &'static str {
    match polymer {
        Polymer::Pla       => "Snapmaker PLA SnapSpeed @U1",
        Polymer::Petg      => "Snapmaker PETG HF",
        Polymer::Abs       => "Generic ABS",
        Polymer::Asa       => "Generic ASA",
        Polymer::Pc        => "Generic PC",
        Polymer::Tpu       => "Generic TPU",
        Polymer::NylonPa6  => "Generic PA6",
        Polymer::NylonPa12 => "Generic PA12",
        Polymer::Hips      => "Generic HIPS",
        Polymer::Pp        => "Generic PP",
        Polymer::Other     => "Generic PLA",
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

    let profile = json!({
        "name":     display,
        "from":     "User",
        "type":     "filament",
        "inherits": inherit_stub_for(polymer),
        "filament_type":      [polymer.as_str()],
        "filament_vendor":    [manufacturer],
        "nozzle_temperature":             to_string_array(ef.nozzle_temp_recommended_c),
        "nozzle_temperature_range_low":   to_string_array(ef.nozzle_temp_min_c),
        "nozzle_temperature_range_high":  to_string_array(ef.nozzle_temp_max_c),
        "hot_plate_temp":                 to_string_array(ef.bed_temp_min_c),
        "hot_plate_temp_initial_layer":   to_string_array(ef.bed_temp_min_c),
        "filament_density":               to_string_array(ef.density_g_cm3),
        "filament_max_volumetric_speed":  to_string_array(ef.max_flow_mm3_s),
        "_leanspectrum_metadata": {
            "source":           "SDS/TDS importer",
            "polymer":          polymer.as_str(),
            "extracted_at":     Utc::now().to_rfc3339(),
            "estimated_fields": ef.estimated_fields,
            "needs_review":     ef.needs_review,
            "source_files":     ef.source_files,
            "revision_date":    ef.revision_date,
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

fn recommend_process(_user_dir: &Path, _polymer: Polymer) -> Option<RecommendedProcess> {
    // Placeholder: real implementation walks the slicer's system/process/
    // directory, parses each .json, filters by the active printer + nozzle
    // size and the polymer's compatible_filaments field, then scores by
    // (print_speed, layer_height). For v0.1 we return None so the UI
    // shows "no recommendation yet" instead of a wrong default.
    None
}
