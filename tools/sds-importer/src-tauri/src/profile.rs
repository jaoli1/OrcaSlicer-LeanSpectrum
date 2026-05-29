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

/// Choose the filament parent for the chosen printer. The Snapmaker U1 keeps its
/// hand-verified U1-tuned parents (best hardware tuning, see `inherit_stub_for`);
/// any OTHER OrcaSlicer-family printer (Creality / Bambu / Prusa …) inherits the
/// broadly-compatible stock "Generic <polymer>" leaf instead, since the
/// "@U1" parents do not exist for those machines (inheriting a missing parent
/// silently breaks the preset — the v0.1.9 lesson).
fn inherit_stub_for_printer(polymer: Polymer, is_u1: bool) -> String {
    if is_u1 {
        inherit_stub_for(polymer).to_string()
    } else {
        polymer.orca_generic_parent().to_string()
    }
}

/// Build a filament profile (name, json) from extracted/DB data for a chosen set
/// of printer presets — the universal one-click path (v0.2.0). `is_u1` selects
/// the U1-tuned parent; otherwise a generic parent is used and the filament is
/// made compatible with exactly the chosen `printers`.
pub fn build_filament_json_for(
    ef: &ExtractedFilament,
    polymer: Polymer,
    printers: &[String],
    is_u1: bool,
    log: &mut Vec<String>,
) -> (String, Value) {
    let inherits = inherit_stub_for_printer(polymer, is_u1);
    let v = build_profile_json(ef, polymer, &inherits, printers, log);
    let name = v["name"].as_str().unwrap_or("Imported filament").to_string();
    (name, v)
}

/// Format a float without trailing zeros: 220.0 -> "220", 1.23 -> "1.23".
/// Stock Snapmaker profiles store temperatures / counts as integer strings
/// and ratios as short decimals; this matches both.
fn fmt_num(x: f64) -> String {
    if x.fract().abs() < 1e-9 { format!("{x:.0}") } else { format!("{x}") }
}

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

/// The printer preset the U1 single-PDF tests target. A user preset is filtered
/// out of the dropdown unless its `compatible_printers` name-matches the active
/// printer (an empty list also passes, but being explicit is safer). Must be the
/// exact preset NAME, not a display alias. v0.3.0: callers now pass the resolved
/// printer name(s), so this is only referenced by the tests.
#[cfg(test)]
const U1_PRINTER: &str = "Snapmaker U1 (0.4 nozzle)";

/// Build the clean filament preset name from the manufacturer + product label.
///
/// v0.3.0 — replaces the old `"{POLYMER} — {product} ({manufacturer})"` scheme
/// (which rendered as the truncated, ugly "PLA _ Eryone PLA_ _She…"). The new
/// rule produces a plain "Brand Material" name:
///   - if `product_name` already starts with `manufacturer` (case-insensitive),
///     use `product_name` as-is; otherwise prefix it: `"{manufacturer} {product}"`,
///   - then strip a leading duplicated polymer token so we never produce
///     "PLA PLA …" (we do NOT prepend the polymer, nor append "(manufacturer)"),
///   - trim and collapse runs of whitespace.
///
/// Examples (see tests):
///   brand "Eryone"    + "Eryone — ABS CF"  → "Eryone ABS CF"
///   brand "Polymaker" + "PolyTerra PLA"    → "Polymaker PolyTerra PLA"
fn filament_display_name(manufacturer: &str, product_name: &str, polymer: Polymer) -> String {
    let manufacturer = manufacturer.trim();
    let product_name = product_name.trim();

    // Base: avoid duplicating the brand when the label already leads with it.
    let starts_with_brand = !manufacturer.is_empty()
        && product_name
            .to_lowercase()
            .starts_with(&manufacturer.to_lowercase());
    let base = if manufacturer.is_empty() || starts_with_brand {
        product_name.to_string()
    } else {
        format!("{manufacturer} {product_name}")
    };

    // Turn em/en dashes (used as separators in labels like "Eryone — ABS CF")
    // into spaces, then collapse whitespace runs → "Eryone ABS CF".
    let base = base.replace(['—', '–'], " ");
    let mut name: String = base.split_whitespace().collect::<Vec<_>>().join(" ");

    // Strip a leading DUPLICATED polymer token so we never produce "PLA PLA …".
    // Only collapses a true repeat (first two whitespace-separated tokens both
    // equal the polymer, case-insensitively) — a single leading "PLA" is kept
    // (so "eSUN PLA" / "Generic PLA Basic" are untouched).
    let poly = polymer.as_str();
    if !poly.is_empty() {
        let mut tokens: Vec<&str> = name.split(' ').collect();
        while tokens.len() >= 2
            && tokens[0].eq_ignore_ascii_case(poly)
            && tokens[1].eq_ignore_ascii_case(poly)
        {
            tokens.remove(0);
        }
        name = tokens.join(" ");
    }

    let name = name.trim().to_string();
    if name.is_empty() {
        product_name.to_string()
    } else {
        name
    }
}

/// Build the filament profile JSON document (pure; no disk I/O). Split out
/// from `build_and_save` so the schema can be unit-tested without touching
/// the user's Snapmaker_Orca directory.
fn build_profile_json(
    ef: &ExtractedFilament,
    polymer: Polymer,
    inherits: &str,
    compatible: &[String],
    log: &mut Vec<String>,
) -> Value {
    let product_name = ef.product_name.as_deref().unwrap_or("Imported filament");
    let manufacturer = ef.manufacturer.as_deref().unwrap_or("Unknown");
    let display      = filament_display_name(manufacturer, product_name, polymer);

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
        "inherits": inherits,
        "compatible_printers": compatible,
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
            "inherit_target":   inherits,
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

        // Per-material cooling / pressure-advance / retraction. The stock
        // parent we inherit from is PLA-flavoured (fan at 100 %, PLA-tuned PA
        // and retraction) which is a thermal mismatch for ABS/ASA/PC/PA — the
        // fan delaminates the part and the retraction strings. These are always
        // emitted (unlike the data-driven scalars above) because the polymer
        // family always yields a value, so there is no risk of blanking a
        // parent key with `[""]`.
        let (fan_min, fan_max, slow_layer_time) = polymer.default_fan_curve();
        let pa                                  = polymer.default_pressure_advance();
        let (retract_len, retract_spd, z_hop)   = polymer.default_retraction();
        let material_tuning: [(&str, String); 8] = [
            ("fan_min_speed",            fmt_num(fan_min)),
            ("fan_max_speed",            fmt_num(fan_max)),
            ("slow_down_layer_time",     fmt_num(slow_layer_time)),
            ("enable_pressure_advance",  "1".to_string()),
            ("pressure_advance",         fmt_num(pa)),
            ("retraction_length",        fmt_num(retract_len)),
            ("retraction_speed",         fmt_num(retract_spd)),
            ("z_hop",                    fmt_num(z_hop)),
        ];
        for (key, v) in material_tuning {
            obj.insert(key.to_string(), json!([v]));
        }
    }

    profile
}


/// Write `value` to `dir/<sanitized name>.json`, appending " (N)" if a file
/// with that name already exists. Shared by the filament and process writers.
pub(crate) fn write_unique_json(dir: &Path, display: &str, value: &Value) -> Result<PathBuf> {
    // Overwrite an existing profile of the same name: re-generating a material
    // refreshes its profile instead of piling up "(1)", "(2)" copies in the
    // slicer's preset list. (The slicer shows the file-name stem, so the stem
    // must equal the intended display name — see `sanitize`.)
    let path = dir.join(format!("{}.json", sanitize(display)));
    fs::write(&path, serde_json::to_string_pretty(value).unwrap())
        .map_err(|e| Error::Profile(e.to_string()))?;
    Ok(path)
}

/// Make `s` safe as a file-name stem WITHOUT mangling legitimate material names.
/// The slicer displays the file-name stem, so we keep characters that are legal
/// in a file name — `+`, `(`, `)`, `,`, `&`… — and replace ONLY the ones that are
/// actually illegal on the common OSes (Windows is strictest: `\ / : * ? " < > |`
/// plus control chars). Trailing dots/spaces are trimmed (illegal on Windows).
/// This is what makes "Eryone PLA+" show as "Eryone PLA+" and not "Eryone PLA_".
fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(|c: char| c == '.' || c == ' ').trim();
    if cleaned.is_empty() { "profile".to_string() } else { cleaned.to_string() }
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
        let v = build_profile_json(
            &eryone_pla(),
            Polymer::Pla,
            inherit_stub_for(Polymer::Pla),
            &[U1_PRINTER.to_string()],
            &mut log,
        );

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

    /// Per-material cooling / pressure-advance / retraction must be written as
    /// filament arrays of strings, and must differ by family: PLA runs the fan
    /// flat-out (fan_max 100) while ABS keeps it low (fan_max 30) to avoid
    /// warping. Both must enable pressure advance. This is the thermal-mismatch
    /// fix: the keys used to be inherited from a PLA-flavoured parent.
    #[test]
    fn per_material_cooling_pa_and_retraction_are_emitted() {
        let mut log = Vec::new();
        let v_pla = build_profile_json(
            &eryone_pla(),
            Polymer::Pla,
            inherit_stub_for(Polymer::Pla),
            &[U1_PRINTER.to_string()],
            &mut log,
        );
        let abs = ExtractedFilament {
            polymer: Some(Polymer::Abs),
            ..Default::default()
        };
        let v_abs = build_profile_json(
            &abs,
            Polymer::Abs,
            inherit_stub_for(Polymer::Abs),
            &[U1_PRINTER.to_string()],
            &mut log,
        );

        // Fan curve diverges by family.
        assert_eq!(v_pla["fan_max_speed"][0], "100");
        assert_eq!(v_abs["fan_max_speed"][0], "30");
        // Pressure advance is enabled for every material.
        assert_eq!(v_pla["enable_pressure_advance"][0], "1");
        assert_eq!(v_abs["enable_pressure_advance"][0], "1");
        // Spot-check the remaining new keys are present as string arrays.
        assert_eq!(v_pla["fan_min_speed"][0],        "100");
        assert_eq!(v_pla["slow_down_layer_time"][0], "8");
        assert_eq!(v_pla["pressure_advance"][0],     "0.02");
        assert_eq!(v_pla["retraction_length"][0],    "0.8");
        assert_eq!(v_pla["retraction_speed"][0],     "30");
        assert_eq!(v_pla["z_hop"][0],                "0");
        assert_eq!(v_abs["retraction_length"][0],    "1");
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
        let v = build_profile_json(
            &ef,
            Polymer::Pla,
            inherit_stub_for(Polymer::Pla),
            &[U1_PRINTER.to_string()],
            &mut log,
        );
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

    /// v0.2.0 universal path: a filament generated for a NON-U1 printer must
    /// inherit a stock "Generic <polymer>" parent and be compatible with exactly
    /// the chosen printer — not the U1. The U1 path keeps its tuned parent.
    #[test]
    fn universal_filament_targets_chosen_printer_with_generic_parent() {
        let mut log = Vec::new();
        let printers = vec!["Creality K1 (0.4 nozzle)".to_string()];
        let (name, v) =
            build_filament_json_for(&eryone_pla(), Polymer::Pla, &printers, false, &mut log);
        // v0.3.0 clean naming: "{brand} {product}" (no "PLA — …(brand)" prefix),
        // and never the truncated "PLA _ Eryone PLA_ _She…".
        assert!(name.contains("Eryone PLA+"), "name was {name}");
        assert!(!name.contains('—'), "no em-dash polymer prefix; name was {name}");
        assert!(!name.contains('('), "no trailing (manufacturer); name was {name}");
        assert_eq!(v["inherits"], "Generic PLA");
        assert_eq!(v["compatible_printers"][0], "Creality K1 (0.4 nozzle)");
        assert_eq!(v["type"], "filament");
        // Data-sheet temperatures are still carried through.
        assert_eq!(v["nozzle_temperature"][0], "210");

        // The U1 path keeps the hand-verified U1-tuned parent + U1 printer.
        let u1 = vec![U1_PRINTER.to_string()];
        let (_, vu) = build_filament_json_for(&eryone_pla(), Polymer::Pla, &u1, true, &mut log);
        assert_eq!(vu["inherits"], "Snapmaker PLA SnapSpeed @U1");
        assert_eq!(vu["compatible_printers"][0], "Snapmaker U1 (0.4 nozzle)");
    }

    /// v0.3.0 clean "Brand Material" naming rule. The old scheme produced the
    /// ugly, truncated "PLA — Eryone PLA (Eryone)" → "PLA _ Eryone PLA_ _She…".
    #[test]
    fn filament_naming_is_clean_brand_material() {
        // Brand already leads the label → label used as-is (no duplication).
        assert_eq!(
            filament_display_name("Eryone", "Eryone — ABS CF", Polymer::Abs),
            "Eryone ABS CF"
        );
        // Brand not in the label → "{brand} {label}".
        assert_eq!(
            filament_display_name("Polymaker", "PolyTerra PLA", Polymer::Pla),
            "Polymaker PolyTerra PLA"
        );
        // Leading DUPLICATED polymer token is collapsed (no "PLA PLA …"); the
        // polymer is NOT prepended and the manufacturer is NOT appended.
        assert_eq!(
            filament_display_name("", "PLA PLA Basic", Polymer::Pla),
            "PLA Basic"
        );
        // A single leading polymer token is KEPT ("Generic PLA Basic" stays).
        assert_eq!(
            filament_display_name("Generic", "PLA Basic", Polymer::Pla),
            "Generic PLA Basic"
        );
        // A label that IS just the polymer survives (not blanked).
        assert_eq!(filament_display_name("eSUN", "PLA", Polymer::Pla), "eSUN PLA");
        // Case-insensitive brand match + whitespace collapse.
        assert_eq!(
            filament_display_name("ERYONE", "eryone   PLA+", Polymer::Pla),
            "eryone PLA+"
        );
        // Empty manufacturer → just the cleaned label.
        assert_eq!(filament_display_name("", "Galaxy PETG", Polymer::Petg), "Galaxy PETG");
        // Never the old format markers.
        let n = filament_display_name("Eryone", "Eryone PLA", Polymer::Pla);
        assert!(!n.contains('—') && !n.contains('('), "name was {n}");
    }

    #[test]
    fn sanitize_keeps_legal_filename_chars() {
        // "+", "(", ")" are legal in a file name and must survive (the slicer
        // shows the stem): "Eryone PLA+" must NOT become "Eryone PLA_".
        assert_eq!(sanitize("Eryone PLA+"), "Eryone PLA+");
        assert_eq!(sanitize("Eryone PLA+ HP"), "Eryone PLA+ HP");
        assert_eq!(sanitize("Silk PLA (Dual Color)"), "Silk PLA (Dual Color)");
        // Only genuinely-illegal characters are replaced.
        assert_eq!(sanitize("a/b:c*d?"), "a_b_c_d_");
        // Trailing dot/space (illegal on Windows) is trimmed.
        assert_eq!(sanitize("Name. "), "Name");
    }
}
