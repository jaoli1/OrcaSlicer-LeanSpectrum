//! v0.1.16 — shared project-type process library.
//!
//! Generates ONE shared set of Snapmaker_Orca *process* profiles named by
//! project type (Vase, Décoration, Figurine, …) for every nozzle diameter
//! (0.2 / 0.4 / 0.6 / 0.8). The per-filament tuning stays in the *filament*
//! profile (temps / flow / PA / retraction) — this is the "one shared set +
//! filament tuning" split.
//!
//! Each profile carries, per project intent and nozzle:
//!   • layer height (scaled to the nozzle, clamped to 25–75 % of its diameter),
//!     wall loops, sparse-infill density + pattern, top/bottom shells,
//!   • print speeds (outer / inner / infill / top),
//!   • cornering & resonance/VFA control via acceleration + jerk limits
//!     (tight & low for Figurine to kill vertical fine artefacts, loose & high
//!     for Prototype),
//!   • vase (spiral) mode and ironing where the intent calls for it,
//!   • the fork features: scarf seams, filament economy, color-mixing
//!     region-collapse (all PROCESS-domain keys the slicer actually applies).
//!
//! All values are emitted as the process-profile convention expects: plain
//! string scalars, bools as "0"/"1". Volumetric-flow capping is NOT done here
//! (it is a filament-domain setting: `filament_max_volumetric_speed`), the U1's
//! 32 mm³/s ceiling lives on the filament side.

use serde_json::{json, Value};

/// Must be a 4-part Semver <= the running slicer's SLIC3R_VERSION or the preset
/// loader silently drops the profile. Kept in sync with version.inc.
const PRESET_VERSION: &str = "01.10.01.70";

/// The four U1 nozzle diameters we generate for.
pub const NOZZLES: [f64; 4] = [0.2, 0.4, 0.6, 0.8];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectType {
    PrototypeRapide,
    ObjetDuQuotidien,
    Figurine,
    Vase,
    Decoration,
    Jouet,
    PieceMecanique,
}

impl ProjectType {
    pub fn all() -> [ProjectType; 7] {
        use ProjectType::*;
        [PrototypeRapide, ObjetDuQuotidien, Figurine, Vase, Decoration, Jouet, PieceMecanique]
    }

    /// Display label used in the profile name (FR, the product language).
    pub fn label(self) -> &'static str {
        use ProjectType::*;
        match self {
            PrototypeRapide => "Prototype rapide",
            ObjetDuQuotidien => "Objet du quotidien",
            Figurine => "Figurine",
            Vase => "Vase",
            Decoration => "Décoration",
            Jouet => "Jouet",
            PieceMecanique => "Pièce mécanique",
        }
    }

    /// Reference parameters at the 0.4 mm nozzle. Per-nozzle values are derived
    /// in [`Self::params_at`].
    fn reference(self) -> Ref {
        use ProjectType::*;
        // layer@0.4, walls, infill%, pattern, (outer,inner,infill,top) mm/s,
        // accel, jerk, spiral, ironing, top_shells, bottom_shells
        match self {
            PrototypeRapide  => Ref { layer: 0.28, walls: 1, infill: 8,  pattern: "grid",      spd: (150.0, 200.0, 250.0, 120.0), accel: 10000.0, jerk: 12.0, spiral: false, ironing: false, top: 3, bot: 3 },
            ObjetDuQuotidien => Ref { layer: 0.20, walls: 3, infill: 15, pattern: "grid",      spd: (120.0, 150.0, 200.0, 100.0), accel: 6000.0,  jerk: 9.0,  spiral: false, ironing: false, top: 4, bot: 4 },
            Figurine         => Ref { layer: 0.12, walls: 3, infill: 15, pattern: "gyroid",    spd: (50.0,  80.0,  100.0, 40.0),  accel: 2000.0,  jerk: 5.0,  spiral: false, ironing: false, top: 5, bot: 4 },
            Vase             => Ref { layer: 0.20, walls: 1, infill: 0,  pattern: "gyroid",    spd: (60.0,  60.0,  60.0,  50.0),  accel: 4000.0,  jerk: 7.0,  spiral: true,  ironing: false, top: 0, bot: 4 },
            Decoration       => Ref { layer: 0.16, walls: 2, infill: 10, pattern: "lightning", spd: (80.0,  120.0, 150.0, 60.0),  accel: 4000.0,  jerk: 7.0,  spiral: false, ironing: true,  top: 5, bot: 4 },
            Jouet            => Ref { layer: 0.20, walls: 4, infill: 30, pattern: "grid",      spd: (100.0, 140.0, 180.0, 80.0),  accel: 6000.0,  jerk: 9.0,  spiral: false, ironing: false, top: 5, bot: 5 },
            PieceMecanique   => Ref { layer: 0.24, walls: 5, infill: 45, pattern: "grid",      spd: (60.0,  100.0, 130.0, 50.0),  accel: 4000.0,  jerk: 7.0,  spiral: false, ironing: false, top: 6, bot: 6 },
        }
    }

    /// Concrete process parameters for this project type at `nozzle` mm.
    pub fn params_at(self, nozzle: f64) -> Params {
        let r = self.reference();
        // Layer height tracks the nozzle (ref is for 0.4) and is clamped to the
        // printable 25–75 % of nozzle-diameter window.
        let scaled = r.layer * (nozzle / 0.4);
        let layer = scaled.clamp(0.25 * nozzle, 0.75 * nozzle);
        let layer = (layer * 100.0).round() / 100.0; // 2 dp
        // First layer a touch thicker for adhesion, never above 75 % nozzle.
        let first = (layer * 1.25).min(0.75 * nozzle);
        let first = (first * 100.0).round() / 100.0;
        Params {
            layer_height: layer,
            initial_layer_height: first,
            wall_loops: r.walls,
            infill_pct: r.infill,
            infill_pattern: r.pattern,
            outer_speed: r.spd.0,
            inner_speed: r.spd.1,
            infill_speed: r.spd.2,
            top_speed: r.spd.3,
            acceleration: r.accel,
            jerk: r.jerk,
            spiral: r.spiral,
            ironing: r.ironing,
            top_shells: r.top,
            bottom_shells: r.bot,
        }
    }
}

struct Ref {
    layer: f64, walls: u32, infill: u32, pattern: &'static str,
    spd: (f64, f64, f64, f64), accel: f64, jerk: f64,
    spiral: bool, ironing: bool, top: u32, bot: u32,
}

#[derive(Clone, Debug)]
pub struct Params {
    pub layer_height: f64,
    pub initial_layer_height: f64,
    pub wall_loops: u32,
    pub infill_pct: u32,
    pub infill_pattern: &'static str,
    pub outer_speed: f64,
    pub inner_speed: f64,
    pub infill_speed: f64,
    pub top_speed: f64,
    pub acceleration: f64,
    pub jerk: f64,
    pub spiral: bool,
    pub ironing: bool,
    pub top_shells: u32,
    pub bottom_shells: u32,
}

fn fmt(x: f64) -> String {
    if x.fract().abs() < 1e-9 { format!("{x:.0}") } else { format!("{x}") }
}

/// A concrete print target for the generator: the printer preset to attach to,
/// the stock base process to inherit, the nozzle, and the printer's max layer
/// height. For the Snapmaker U1 these come from `u1_spec`; for ANY other
/// OrcaSlicer-family printer (Creality / Bambu / Snapmaker / …) they come from
/// the machine catalogue (vendor → model → variant), which is how the same 7
/// project-type profiles are produced for every supported machine.
#[derive(Clone, Debug)]
pub struct PrinterSpec {
    pub printer_name: String,
    pub base_process: String,
    pub nozzle: f64,
    pub max_layer_height: f64,
}

fn u1_spec(nozzle: f64) -> PrinterSpec {
    let (printer_name, base_process) = nozzle_targets(nozzle);
    PrinterSpec { printer_name, base_process, nozzle, max_layer_height: 0.75 * nozzle }
}

/// The printer preset + stock base process to inherit, per nozzle. These names
/// MUST match the U1 per-nozzle profiles that ship in the slicer (created with
/// the 0.2/0.4/0.6/0.8 printer + "0.10/0.20/0.30/0.40 Standard" base process).
fn nozzle_targets(nozzle: f64) -> (String, String) {
    let printer = format!("Snapmaker U1 ({} nozzle)", fmt(nozzle));
    // First-layer height of the stock base ladders 0.10/0.20/0.30/0.40.
    let base_lh = match nozzle {
        n if (n - 0.2).abs() < 1e-9 => "0.10",
        n if (n - 0.4).abs() < 1e-9 => "0.20",
        n if (n - 0.6).abs() < 1e-9 => "0.30",
        _ => "0.40",
    };
    let base_process = format!("{base_lh} Standard @Snapmaker U1 ({} nozzle)", fmt(nozzle));
    (printer, base_process)
}

/// Build a single project-type process profile for the Snapmaker U1 at `nozzle`.
pub fn build_one(pt: ProjectType, nozzle: f64) -> (String, Value) {
    build_one_for(pt, &u1_spec(nozzle))
}

/// Build a single project-type process profile for ANY OrcaSlicer-family printer
/// described by `spec` (printer preset + stock base process to inherit). The
/// reference parameters are scaled to the nozzle and clamped to the printer's
/// max layer height.
pub fn build_one_for(pt: ProjectType, spec: &PrinterSpec) -> (String, Value) {
    let mut p = pt.params_at(spec.nozzle);
    if spec.max_layer_height > 0.0 {
        if p.layer_height > spec.max_layer_height {
            p.layer_height = spec.max_layer_height;
        }
        if p.initial_layer_height > spec.max_layer_height {
            p.initial_layer_height = spec.max_layer_height;
        }
    }
    let printer = spec.printer_name.clone();
    let base_process = spec.base_process.clone();
    let name = format!("{} @{}", pt.label(), spec.printer_name);

    let mut v = json!({
        "name": name,
        "version": PRESET_VERSION,
        "from": "User",
        "is_custom_defined": "1",
        "type": "process",
        "inherits": base_process,
        "compatible_printers": [printer],

        "layer_height":               fmt(p.layer_height),
        "initial_layer_print_height": fmt(p.initial_layer_height),
        "wall_loops":                 p.wall_loops.to_string(),
        "top_shell_layers":           p.top_shells.to_string(),
        "bottom_shell_layers":        p.bottom_shells.to_string(),
        "sparse_infill_density":      format!("{}%", p.infill_pct),
        "sparse_infill_pattern":      p.infill_pattern,

        "outer_wall_speed":           fmt(p.outer_speed),
        "inner_wall_speed":           fmt(p.inner_speed),
        "sparse_infill_speed":        fmt(p.infill_speed),
        "internal_solid_infill_speed":fmt(p.infill_speed),
        "top_surface_speed":          fmt(p.top_speed),

        // cornering + resonance / VFA control: low accel & jerk = tight corners,
        // fewer vertical fine artefacts (Figurine); high = fast (Prototype).
        "default_acceleration":       fmt(p.acceleration),
        "outer_wall_acceleration":    fmt((p.acceleration * 0.5).round()),
        "inner_wall_acceleration":    fmt(p.acceleration),
        "top_surface_acceleration":   fmt((p.acceleration * 0.5).round()),
        "travel_acceleration":        fmt((p.acceleration * 1.25).round()),
        "default_jerk":               fmt(p.jerk),
        "outer_wall_jerk":            fmt(p.jerk),
        "inner_wall_jerk":            fmt(p.jerk),
        "top_surface_jerk":           fmt((p.jerk * 0.7).round().max(1.0)),
        "travel_jerk":                fmt((p.jerk * 1.5).round()),

        // --- fork features (PROCESS-domain; same set the per-import companion emits) ---
        "filament_economy_enable":            "1",
        "filament_economy_remove_noop_swaps": "1",
        "filament_economy_shrink_purge":      "1",
        "filament_economy_shrink_purge_pct":  "30",
        "filament_economy_curvature_lh":      "1",
        "filament_economy_force_m83":         "1",
        "mixed_filament_region_collapse":     "1",

        "_leanspectrum_metadata": {
            "source": "Optimisateur — bibliothèque process par type de projet (multi-imprimante)",
            "project_type": pt.label(),
            "printer": spec.printer_name,
            "nozzle_mm": spec.nozzle,
            "base_process": base_process,
        }
    });

    {
        let obj = v.as_object_mut().expect("process root is an object");
        if p.spiral {
            // Vase mode: single wall, no top, hollow.
            obj.insert("spiral_mode".into(), json!("1"));
            obj.insert("wall_loops".into(), json!("1"));
            obj.insert("top_shell_layers".into(), json!("0"));
            obj.insert("sparse_infill_density".into(), json!("0%"));
        }
        if p.ironing {
            obj.insert("ironing_type".into(), json!("topmost"));
            obj.insert("ironing_flow".into(), json!("10%"));
            obj.insert("ironing_speed".into(), json!(fmt(p.top_speed.min(30.0))));
        }
        // Scarf seams: on for the surface-quality intents, off where it adds no
        // value (draft) or is incompatible (vase = single continuous wall).
        let scarf = matches!(
            pt,
            ProjectType::ObjetDuQuotidien | ProjectType::Figurine
                | ProjectType::Decoration | ProjectType::Jouet | ProjectType::PieceMecanique
        );
        if scarf {
            obj.insert("seam_slope_type".into(), json!("external"));
            obj.insert("seam_slope_conditional".into(), json!("1"));
            obj.insert("scarf_angle_threshold".into(), json!("155"));
            obj.insert("scarf_joint_speed".into(), json!("100%"));
            obj.insert("scarf_joint_flow_ratio".into(), json!("1"));
            obj.insert("seam_slope_min_length".into(), json!("10"));
            obj.insert("seam_slope_steps".into(), json!("10"));
        }
    }
    (name, v)
}

/// The full shared library for the Snapmaker U1: every project type × every
/// nozzle (7 × 4 = 28).
pub fn build_library() -> Vec<(String, Value)> {
    let mut out = Vec::with_capacity(ProjectType::all().len() * NOZZLES.len());
    for pt in ProjectType::all() {
        for n in NOZZLES {
            out.push(build_one(pt, n));
        }
    }
    out
}

/// On-demand multi-printer path: the 7 project-type profiles for an arbitrary
/// set of printer specs (one per chosen printer/nozzle variant resolved from the
/// machine catalogue). Same engine as the U1 library — this is what makes the
/// optimiser cover every OrcaSlicer-family printer, not just the U1.
pub fn build_library_for(specs: &[PrinterSpec]) -> Vec<(String, Value)> {
    let mut out = Vec::with_capacity(ProjectType::all().len() * specs.len());
    for s in specs {
        for pt in ProjectType::all() {
            out.push(build_one_for(pt, s));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_for_any_orca_family_printer() {
        // The same engine targets any catalogue printer, not just the U1.
        let k1 = PrinterSpec {
            printer_name: "Creality K1 (0.4 nozzle)".into(),
            base_process: "0.20mm Standard @Creality K1".into(),
            nozzle: 0.4,
            max_layer_height: 0.30,
        };
        let (name, v) = build_one_for(ProjectType::PieceMecanique, &k1);
        assert_eq!(name, "Pièce mécanique @Creality K1 (0.4 nozzle)");
        assert_eq!(v["inherits"], "0.20mm Standard @Creality K1");
        assert_eq!(v["compatible_printers"][0], "Creality K1 (0.4 nozzle)");
        assert_eq!(v["filament_economy_enable"], "1");
        // One spec yields the 7 project types.
        assert_eq!(build_library_for(&[k1.clone()]).len(), 7);
        // A printer whose max layer height is 0.20 caps the thick draft layer.
        let capped = PrinterSpec { max_layer_height: 0.20, ..k1 };
        let (_, vp) = build_one_for(ProjectType::PrototypeRapide, &capped);
        assert_eq!(vp["layer_height"], "0.2");
    }

    #[test]
    fn library_has_28_unique_named_profiles() {
        let lib = build_library();
        assert_eq!(lib.len(), 28);
        let mut names: Vec<&str> = lib.iter().map(|(n, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 28, "profile names must be unique");
    }

    #[test]
    fn every_profile_is_registrable_and_targets_its_nozzle() {
        for (name, v) in build_library() {
            assert_eq!(v["version"], PRESET_VERSION);
            assert_eq!(v["type"], "process");
            assert_eq!(v["is_custom_defined"], "1");
            // compatible printer + inherited base must reference the same nozzle.
            let printer = v["compatible_printers"][0].as_str().unwrap();
            let base = v["inherits"].as_str().unwrap();
            for nz in ["0.2", "0.4", "0.6", "0.8"] {
                if name.contains(&format!("({nz} nozzle)")) {
                    assert!(printer.contains(nz), "{name}: printer {printer}");
                    assert!(base.contains(nz), "{name}: base {base}");
                }
            }
        }
    }

    #[test]
    fn layer_height_within_25_75_pct_of_nozzle() {
        for pt in ProjectType::all() {
            for n in NOZZLES {
                let p = pt.params_at(n);
                assert!(p.layer_height >= 0.25 * n - 1e-6 && p.layer_height <= 0.75 * n + 1e-6,
                        "{:?}@{n}: layer {} outside 25-75% of nozzle", pt, p.layer_height);
            }
        }
    }

    #[test]
    fn vase_is_spiral_and_figurine_is_low_resonance() {
        let (_, vase) = build_one(ProjectType::Vase, 0.4);
        assert_eq!(vase["spiral_mode"], "1");
        assert_eq!(vase["wall_loops"], "1");
        assert_eq!(vase["sparse_infill_density"], "0%");

        // Figurine: tightest cornering (lowest accel & jerk of the set).
        let fig = ProjectType::Figurine.params_at(0.4);
        let proto = ProjectType::PrototypeRapide.params_at(0.4);
        assert!(fig.acceleration < proto.acceleration);
        assert!(fig.jerk < proto.jerk);

        // Décoration enables ironing; Pièce mécanique stacks walls.
        let (_, deco) = build_one(ProjectType::Decoration, 0.4);
        assert_eq!(deco["ironing_type"], "topmost");
        assert_eq!(ProjectType::PieceMecanique.params_at(0.4).wall_loops, 5);
    }

    #[test]
    fn fork_features_on_every_profile() {
        for (name, v) in build_library() {
            assert_eq!(v["filament_economy_enable"], "1", "{name}");
            assert_eq!(v["mixed_filament_region_collapse"], "1", "{name}");
        }
    }
}
