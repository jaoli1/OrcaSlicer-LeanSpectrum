//! Machine catalogue — resolves a chosen vendor / model / nozzle to a
//! [`PrinterSpec`] so the process generator covers EVERY OrcaSlicer-family
//! printer (Creality, Bambu, Snapmaker, Anycubic, …), not just the U1.
//!
//! The catalogue (57 vendors / 326 models / 796 variants) is embedded at compile
//! time from `data/machine_catalog.json` (built by scripts/build_machine_catalog.py
//! out of the bundled OrcaSlicer profiles). Each variant carries the printer's
//! `default_process_name` (e.g. "0.20mm Standard @Creality K1") — its part after
//! the last '@' is the printer-preset name to target, and the whole string is the
//! stock base process to inherit.

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::project_process::PrinterSpec;

const CATALOG_JSON: &str = include_str!("../../data/machine_catalog.json");

#[derive(Deserialize)]
struct Catalog {
    vendors: Vec<Vendor>,
    machines: Vec<Machine>,
    machine_variants: Vec<Variant>,
}
#[derive(Deserialize)]
struct Vendor {
    id: i64,
    name: String,
}
#[derive(Deserialize)]
struct Machine {
    id: i64,
    vendor_id: i64,
    model_name: String,
}
#[derive(Deserialize)]
struct Variant {
    machine_id: i64,
    nozzle_diameter: f64,
    #[serde(default)]
    max_layer_height: Option<f64>,
    #[serde(default)]
    default_process_name: Option<String>,
}

static CATALOG: Lazy<Catalog> =
    Lazy::new(|| serde_json::from_str(CATALOG_JSON).expect("machine_catalog.json is valid JSON"));

/// Sorted, de-duplicated vendor names.
pub fn vendors() -> Vec<String> {
    let mut v: Vec<String> = CATALOG.vendors.iter().map(|x| x.name.clone()).collect();
    v.sort();
    v.dedup();
    v
}

fn vendor_id(vendor: &str) -> Option<i64> {
    CATALOG.vendors.iter().find(|v| v.name == vendor).map(|v| v.id)
}

fn machine_ids(vendor: &str, model: &str) -> Vec<i64> {
    let Some(vid) = vendor_id(vendor) else { return vec![] };
    CATALOG
        .machines
        .iter()
        .filter(|m| m.vendor_id == vid && m.model_name == model)
        .map(|m| m.id)
        .collect()
}

/// Sorted, de-duplicated model names for a vendor.
pub fn models(vendor: &str) -> Vec<String> {
    let Some(vid) = vendor_id(vendor) else { return vec![] };
    let mut m: Vec<String> = CATALOG
        .machines
        .iter()
        .filter(|m| m.vendor_id == vid)
        .map(|m| m.model_name.clone())
        .collect();
    m.sort();
    m.dedup();
    m
}

/// Available nozzle diameters for a model (sorted ascending).
pub fn nozzles(vendor: &str, model: &str) -> Vec<f64> {
    let mids = machine_ids(vendor, model);
    let mut n: Vec<f64> = CATALOG
        .machine_variants
        .iter()
        .filter(|v| mids.contains(&v.machine_id))
        .map(|v| v.nozzle_diameter)
        .collect();
    n.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    n.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    n
}

/// Resolve (vendor, model, nozzle) → a PrinterSpec. `nozzle = None` picks the
/// 0.4 mm variant if present, else the smallest.
pub fn resolve(vendor: &str, model: &str, nozzle: Option<f64>) -> Option<PrinterSpec> {
    let mids = machine_ids(vendor, model);
    let mut cands: Vec<&Variant> = CATALOG
        .machine_variants
        .iter()
        .filter(|v| mids.contains(&v.machine_id) && v.default_process_name.is_some())
        .collect();
    if cands.is_empty() {
        return None;
    }
    cands.sort_by(|a, b| a.nozzle_diameter.partial_cmp(&b.nozzle_diameter).unwrap_or(std::cmp::Ordering::Equal));
    let v = match nozzle {
        Some(n) => *cands.iter().find(|v| (v.nozzle_diameter - n).abs() < 1e-6)?,
        None => *cands
            .iter()
            .find(|v| (v.nozzle_diameter - 0.4).abs() < 1e-6)
            .unwrap_or(&cands[0]),
    };
    // Printer-preset name = the segment after the last '@' in the default
    // process name; fall back to the model name if the base has no '@'.
    let base_process = v.default_process_name.clone().unwrap(); // safe: filtered is_some
    let printer_name = base_process
        .rsplit('@')
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.as_str() != base_process.as_str())
        .unwrap_or_else(|| model.to_string());
    let max_lh = match v.max_layer_height {
        Some(x) if x > 0.0 => x,
        _ => 0.75 * v.nozzle_diameter,
    };
    Some(PrinterSpec {
        printer_name,
        base_process,
        nozzle: v.nozzle_diameter,
        max_layer_height: max_lh,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_parses_and_resolves() {
        assert!(vendors().len() >= 40, "expected many vendors");
        // Afinia H+1(HS) 0.4 is variant[0] in the catalogue.
        let spec = resolve("Afinia", "Afinia H+1(HS)", Some(0.4)).expect("resolve Afinia 0.4");
        assert_eq!(spec.printer_name, "Afinia H+1(HS)");
        assert_eq!(spec.base_process, "0.20mm Standard @Afinia H+1(HS)");
        assert_eq!(spec.nozzle, 0.4);
        assert!(spec.max_layer_height > 0.0);
        // models + nozzles are populated for a known vendor.
        assert!(!models("Creality").is_empty());
    }
}
