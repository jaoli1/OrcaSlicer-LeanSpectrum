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

use crate::architecture::Architecture;

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
    FigurineArticulee,
    Vase,
    Decoration,
    Jouet,
    PieceMecanique,
}

impl ProjectType {
    pub fn all() -> [ProjectType; 8] {
        use ProjectType::*;
        [PrototypeRapide, ObjetDuQuotidien, Figurine, FigurineArticulee, Vase, Decoration, Jouet, PieceMecanique]
    }

    /// Display label used in the profile name (FR, the product language).
    pub fn label(self) -> &'static str {
        use ProjectType::*;
        match self {
            PrototypeRapide => "Prototype rapide",
            ObjetDuQuotidien => "Objet du quotidien",
            Figurine => "Figurine",
            FigurineArticulee => "Figurine articulée",
            Vase => "Vase",
            Decoration => "Décoration",
            Jouet => "Jouet",
            PieceMecanique => "Pièce mécanique",
        }
    }

    /// v0.3.1/v0.4.1 — bed adhesion. A modest outer brim on the functional,
    /// larger-footprint intents (Objet du quotidien, Jouet, Pièce mécanique) and
    /// a smaller one on Figurine (small feet need it); none on Vase / Décoration
    /// / the quick Prototype. A shared process can't be per-material, so adhesion
    /// keys off the project type.
    fn wants_brim(self) -> bool {
        use ProjectType::*;
        matches!(self, ObjetDuQuotidien | Jouet | PieceMecanique | Figurine)
    }

    /// v0.4.1 — auto-support (by overhang threshold) on the intents that commonly
    /// have steep overhangs. Never on Vase (spiral mode forbids `enable_support`)
    /// or the quick Prototype. Threshold-based: support is generated only where
    /// the model actually overhangs, not everywhere.
    fn wants_support(self) -> bool {
        use ProjectType::*;
        matches!(self, Figurine | Jouet | PieceMecanique)
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
            Figurine         => Ref { layer: 0.12, walls: 3, infill: 10, pattern: "gyroid",    spd: (75.0,  80.0,  100.0, 40.0),  accel: 4500.0,  jerk: 8.0,  spiral: false, ironing: false, top: 4, bot: 4 },
            // Articulated / print-in-place: NO support (would fuse the joints),
            // NO brim. Moderate 0.16 layer for clean joint clearance, 2 walls,
            // light 10 % infill, moderate speed/accel for crisp small joints.
            FigurineArticulee=> Ref { layer: 0.16, walls: 2, infill: 10, pattern: "grid",      spd: (60.0,  100.0, 150.0, 50.0),  accel: 3500.0,  jerk: 7.0,  spiral: false, ironing: false, top: 4, bot: 4 },
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
    /// Multi-material architecture (drives the purge-tower keys). The U1 is a
    /// 4-nozzle tool-changer → `MultiNozzle`; catalogue printers are classified
    /// in `catalog::resolve` via `architecture::classify`.
    pub architecture: Architecture,
    /// Printer `machine_max_jerk` for X/Y, in mm/s. Emitted process jerk is
    /// clamped to this so the slicer never warns "jerk exceeds machine maximum"
    /// and silently auto-caps — which looks broken in a finished product. The
    /// Snapmaker U1 caps X/Y at 9 mm/s (resources/.../fdm_toolchanger.json);
    /// `DEFAULT_MAX_JERK` is the conservative catalogue fallback. A value of 0
    /// (or negative) disables clamping.
    pub max_jerk: f64,
}

/// Conservative `machine_max_jerk` (X/Y, mm/s) used when we have no per-machine
/// value. 9 mm/s matches the Snapmaker U1 and the common Bambu/Prusa default,
/// and is high enough that clamping to it never degrades print quality (jerk
/// governs cornering, not straight-line throughput — the acceleration limits
/// drive that).
pub const DEFAULT_MAX_JERK: f64 = 9.0;

fn u1_spec(nozzle: f64) -> PrinterSpec {
    let (printer_name, base_process) = nozzle_targets(nozzle);
    PrinterSpec {
        printer_name,
        base_process,
        nozzle,
        max_layer_height: 0.75 * nozzle,
        architecture: Architecture::MultiNozzle,
        // The U1 caps X/Y jerk at 9 mm/s (fdm_toolchanger.json, inherited by
        // every nozzle variant).
        max_jerk: 9.0,
    }
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
pub fn build_one(pt: ProjectType, nozzle: f64, ams_enabled: bool) -> (String, Value) {
    build_one_for(pt, &u1_spec(nozzle), ams_enabled)
}

/// Build a single project-type process profile for ANY OrcaSlicer-family printer
/// described by `spec` (printer preset + stock base process to inherit). The
/// reference parameters are scaled to the nozzle and clamped to the printer's
/// max layer height.
///
/// `ams_enabled` is the UI "I use an AMS / CFS / MMU" checkbox. The print is
/// effectively multi-material when the printer is a tool-changer (`MultiNozzle`)
/// OR it is an `AmsCapable` printer and the user enabled the add-on. Only then
/// do we emit the PURGE / wipe-tower economy keys — a plain single-material
/// print never builds a tower, so carrying those keys would be misleading.
pub fn build_one_for(pt: ProjectType, spec: &PrinterSpec, ams_enabled: bool) -> (String, Value) {
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

    // Effective multi-material: a tool-changer is ALWAYS multi-material; an
    // AMS/CFS/MMU-capable printer only when the user ticked the box. A plain
    // single-nozzle printer (most of the catalogue) is never multi-material.
    let is_mm = matches!(spec.architecture, Architecture::MultiNozzle)
        || (matches!(spec.architecture, Architecture::AmsCapable) && ams_enabled);

    // The slicer warns ("le réglage du jerk dépasse le jerk maximum de
    // l'imprimante") and silently auto-caps when a process jerk exceeds the
    // target printer's machine_max_jerk — ugly in a finished product. We clamp
    // every emitted jerk to the machine ceiling here. The reference table keeps
    // its higher values as relative *cornering intent* (Prototype loose →
    // Figurine tight); we only ever lower them to fit the real machine. Travel
    // jerk (normally 1.5× print jerk) is clamped to the same ceiling too —
    // non-extruding moves gain nothing from extra jerk but ring more. A
    // max_jerk of 0 disables clamping (treat the machine as unbounded).
    let jmax = if spec.max_jerk > 0.0 { spec.max_jerk } else { f64::INFINITY };
    let jclamp = |x: f64| fmt(x.min(jmax).max(1.0));

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
        "default_jerk":               jclamp(p.jerk),
        "outer_wall_jerk":            jclamp(p.jerk),
        "inner_wall_jerk":            jclamp(p.jerk),
        "top_surface_jerk":           jclamp((p.jerk * 0.7).round()),
        "travel_jerk":                jclamp((p.jerk * 1.5).round()),

        // NB: the PURGE / wipe-tower economy keys are emitted further down, but
        // ONLY when the print is effectively multi-material (`is_mm`). A plain
        // single-material print never builds a tower, so those keys are dropped.

        // supports: grip the plate, peel cleanly off the MODEL (geometric)
        "support_top_z_distance":       "0.2",
        "support_bottom_z_distance":    "0.2",
        "support_interface_spacing":    "0.5",
        "support_interface_pattern":    "rectilinear",
        "support_interface_top_layers": "2",
        "support_object_xy_distance":   "0.35",

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
            ProjectType::ObjetDuQuotidien | ProjectType::Figurine | ProjectType::FigurineArticulee
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
        // v0.3.1/v0.4.1 — project-type bed adhesion. Outer brim on functional,
        // larger-footprint intents (5 mm) and a smaller one on Figurine (3 mm,
        // for small feet); explicit no_brim on Vase / Décoration / Prototype.
        if pt.wants_brim() {
            let brim_w = if matches!(pt, ProjectType::Figurine) { "3" } else { "5" };
            obj.insert("brim_type".into(), json!("outer_only"));
            obj.insert("brim_width".into(), json!(brim_w));
            obj.insert("brim_object_gap".into(), json!("0.1"));
        } else {
            obj.insert("brim_type".into(), json!("no_brim"));
        }
        // v0.4.1/v0.6.1 — auto-support on overhang-prone intents only, with the
        // support TYPE matched to the geometry:
        //   • Figurine / Jouet (organic, curvy) → ORGANIC TREE: far less
        //     material, fewer surface marks, easier removal, and it does not
        //     encase the model the way a normal grid does.
        //   • Pièce mécanique (flat, planar overhangs) → NORMAL (snug): tighter
        //     and more predictable on large flat overhangs than tree.
        // support_threshold_angle is INVERTED from intuition in OrcaSlicer: the
        // self-support angle = 90° − (threshold+1) (Support/TreeSupportCommon.hpp),
        // so a HIGHER threshold supports MILDER overhangs → MORE support. Figurine
        // therefore uses a LOW 30° (self-support ≈59° → only pronounced overhangs
        // get support — far lighter; a display piece tolerates a little underside
        // droop). Jouet / Pièce mécanique keep 45° for reliable support on real
        // overhangs (kids' toys, dimensional parts).
        if pt.wants_support() {
            let (s_type, s_style) = if matches!(pt, ProjectType::PieceMecanique) {
                ("normal(auto)", "snug")
            } else {
                ("tree(auto)", "organic")
            };
            let s_threshold = if matches!(pt, ProjectType::Figurine) { "30" } else { "45" };
            obj.insert("enable_support".into(), json!("1"));
            obj.insert("support_type".into(), json!(s_type));
            obj.insert("support_style".into(), json!(s_style));
            obj.insert("support_threshold_angle".into(), json!(s_threshold));
        } else if matches!(pt, ProjectType::FigurineArticulee) {
            // Articulated / print-in-place figurines: support is FORBIDDEN — it
            // would fuse the joints. Force it OFF explicitly so a base profile
            // that defaults support on can't sneak it back in.
            obj.insert("enable_support".into(), json!("0"));
        }

        // --- PURGE / wipe-tower economy — multi-material ONLY. These keys only
        //     mean anything when the slicer actually builds a purge/wipe tower,
        //     i.e. a tool-changer (`MultiNozzle`) or an AMS/CFS/MMU print the
        //     user enabled (`AmsCapable` + checkbox). On a plain single-material
        //     print there is no tower, so we leave them off entirely instead of
        //     carrying misleading no-ops.
        if is_mm {
            // --- UNIVERSAL filament economy (works on EVERY OrcaSlicer/Bambu-
            //     family slicer, not just our fork — and, unlike the fork's post-
            //     export pass, these are slicing-time decisions so the gain shows
            //     up in the PREVIEW and the sliced filament/time estimate).
            //   • flush_into_infill: recycle the colour-change purge into the
            //     model's infill instead of dumping it on the wipe tower.
            //   • flush_into_support: recycle purge into support (already default-
            //     on upstream; set explicitly so a stale base can't disable it).
            //   • wipe_tower_no_sparse_layers: drop the wipe tower's sparse filler
            //     layers — shrinks the tower itself, the biggest waste source.
            //   NOTE: flush_into_objects is deliberately left OFF — routing purge
            //   into the object body bleeds the previous colour onto the surface.
            obj.insert("flush_into_infill".into(), json!("1"));
            obj.insert("flush_into_support".into(), json!("1"));
            obj.insert("wipe_tower_no_sparse_layers".into(), json!("1"));

            // prime_volume scales with nozzle bore: a coarser nozzle holds more
            // ooze to clear at each tool change, so a flat value either over-
            // primes fine nozzles or UNDER-primes coarse ones (colour bleed on
            // the first segment after a change). ≈38×nozzle anchors the validated
            // 0.4 mm → 15 mm³ (0.2→8, 0.6→23, 0.8→30); floor 4 mm³.
            let prime_volume = ((spec.nozzle * 38.0).round().max(4.0) as i64).to_string();
            // Two purge levers, because they govern DIFFERENT printer families:
            //   • prime_volume — THE lever on tool-changer printers like the
            //     Snapmaker U1 (4 independent nozzles). There OrcaSlicer discards
            //     flush_multiplier/flush_volumes and lays exactly `prime_volume`
            //     per tool, per tool-change layer (Print.cpp ~3125 / ~3347). Each
            //     nozzle keeps its own colour, so the prime only clears ooze.
            //     ALSO fires on single-nozzle MM printers whose tower isn't a
            //     purge tower (Creality CFS, Flashforge AD5X, Prusa MMU3 with
            //     single_extruder_multi_material + purge_in_prime_tower=0).
            //   • flush_multiplier — the lever on single-nozzle AMS / purge-in-
            //     tower printers (Bambu — hard-gated to this path — Qidi, etc.):
            //     scales each colour-change purge. 0.2 (vs 0.3 default) trims ~⅓,
            //     above the ~0.15 colour-bleed floor. Inert on tool-changers, so
            //     we set both.
            //   • prime_tower_width caps the tower footprint (30 mm).
            obj.insert("prime_volume".into(), json!(prime_volume));
            obj.insert("flush_multiplier".into(), json!("0.2"));
            obj.insert("prime_tower_width".into(), json!("30"));

            // --- fork features (PROCESS-domain; same set the per-import companion
            //     emits). These add a further post-export pass on our fork only;
            //     harmless (ignored) keys on stock slicers.
            obj.insert("filament_economy_enable".into(), json!("1"));
            obj.insert("filament_economy_remove_noop_swaps".into(), json!("1"));
            obj.insert("filament_economy_shrink_purge".into(), json!("1"));
            obj.insert("filament_economy_shrink_purge_pct".into(), json!("30"));
            obj.insert("filament_economy_curvature_lh".into(), json!("1"));
            obj.insert("filament_economy_force_m83".into(), json!("1"));
            obj.insert("mixed_filament_region_collapse".into(), json!("1"));
        }
    }
    (name, v)
}

/// The full shared library for the Snapmaker U1: every project type × every
/// nozzle (8 × 4 = 32). The U1 is a tool-changer (`MultiNozzle`) so it is always
/// multi-material regardless of `ams_enabled`; the param is threaded for a
/// uniform signature.
pub fn build_library(ams_enabled: bool) -> Vec<(String, Value)> {
    let mut out = Vec::with_capacity(ProjectType::all().len() * NOZZLES.len());
    for pt in ProjectType::all() {
        for n in NOZZLES {
            out.push(build_one(pt, n, ams_enabled));
        }
    }
    out
}

/// On-demand multi-printer path: the 8 project-type profiles for an arbitrary
/// set of printer specs (one per chosen printer/nozzle variant resolved from the
/// machine catalogue). Same engine as the U1 library — this is what makes the
/// optimiser cover every OrcaSlicer-family printer, not just the U1.
pub fn build_library_for(specs: &[PrinterSpec], ams_enabled: bool) -> Vec<(String, Value)> {
    let mut out = Vec::with_capacity(ProjectType::all().len() * specs.len());
    for s in specs {
        for pt in ProjectType::all() {
            out.push(build_one_for(pt, s, ams_enabled));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_for_any_orca_family_printer() {
        // The same engine targets any catalogue printer, not just the U1. The
        // K1 has a CFS → AmsCapable; we generate with ams_enabled=true so the
        // economy keys are present for the assertions below.
        let k1 = PrinterSpec {
            printer_name: "Creality K1 (0.4 nozzle)".into(),
            base_process: "0.20mm Standard @Creality K1".into(),
            nozzle: 0.4,
            max_layer_height: 0.30,
            architecture: Architecture::AmsCapable,
            max_jerk: DEFAULT_MAX_JERK,
        };
        let (name, v) = build_one_for(ProjectType::PieceMecanique, &k1, true);
        assert_eq!(name, "Pièce mécanique @Creality K1 (0.4 nozzle)");
        assert_eq!(v["inherits"], "0.20mm Standard @Creality K1");
        assert_eq!(v["compatible_printers"][0], "Creality K1 (0.4 nozzle)");
        assert_eq!(v["filament_economy_enable"], "1");
        // One spec yields the 8 project types.
        assert_eq!(build_library_for(&[k1.clone()], true).len(), 8);
        // A printer whose max layer height is 0.20 caps the thick draft layer.
        let capped = PrinterSpec { max_layer_height: 0.20, ..k1 };
        let (_, vp) = build_one_for(ProjectType::PrototypeRapide, &capped, true);
        assert_eq!(vp["layer_height"], "0.2");
    }

    #[test]
    fn purge_tower_keys_are_multi_material_only() {
        use ProjectType::*;
        const ECONOMY_KEYS: &[&str] = &[
            "prime_volume", "flush_multiplier", "prime_tower_width",
            "flush_into_infill", "flush_into_support", "wipe_tower_no_sparse_layers",
            "filament_economy_enable", "filament_economy_remove_noop_swaps",
            "filament_economy_shrink_purge", "filament_economy_shrink_purge_pct",
            "filament_economy_curvature_lh", "filament_economy_force_m83",
            "mixed_filament_region_collapse",
        ];
        let mk = |arch| PrinterSpec {
            printer_name: "Test (0.4 nozzle)".into(),
            base_process: "0.20mm Standard @Test".into(),
            nozzle: 0.4,
            max_layer_height: 0.30,
            architecture: arch,
            max_jerk: DEFAULT_MAX_JERK,
        };
        // Single-nozzle: NEVER carries the tower keys (no checkbox can change it).
        let single = mk(Architecture::Single);
        for ae in [false, true] {
            let (_, v) = build_one_for(ObjetDuQuotidien, &single, ae);
            for k in ECONOMY_KEYS {
                assert!(v.get(*k).is_none(), "Single (ams={ae}) must NOT carry {k}");
            }
        }
        // AmsCapable + ams_enabled=false: also no tower keys (add-on not in use).
        let ams = mk(Architecture::AmsCapable);
        let (_, off) = build_one_for(ObjetDuQuotidien, &ams, false);
        for k in ECONOMY_KEYS {
            assert!(off.get(*k).is_none(), "AmsCapable+false must NOT carry {k}");
        }
        // MultiNozzle (any ams flag) and AmsCapable+true MUST carry them.
        let multi = mk(Architecture::MultiNozzle);
        for (label, v) in [
            ("MultiNozzle+false", build_one_for(ObjetDuQuotidien, &multi, false).1),
            ("MultiNozzle+true", build_one_for(ObjetDuQuotidien, &multi, true).1),
            ("AmsCapable+true", build_one_for(ObjetDuQuotidien, &ams, true).1),
        ] {
            for k in ECONOMY_KEYS {
                assert!(v.get(*k).is_some(), "{label} MUST carry {k}");
            }
            // prime_volume scaled ≈38×nozzle (0.4 → 15).
            assert_eq!(v["prime_volume"], "15", "{label} prime_volume");
        }
    }

    #[test]
    fn brim_and_support_keys_follow_project_type() {
        use ProjectType::*;
        // Functional, larger-footprint intents get a 5 mm outer brim…
        for pt in [ObjetDuQuotidien, Jouet, PieceMecanique] {
            let (_, v) = build_one(pt, 0.4, true);
            assert_eq!(v["brim_type"], "outer_only", "{pt:?} should brim");
            assert_eq!(v["brim_width"], "5", "{pt:?} brim width");
        }
        // …Figurine a smaller 3 mm brim (small feet)…
        let (_, fig) = build_one(Figurine, 0.4, true);
        assert_eq!(fig["brim_type"], "outer_only");
        assert_eq!(fig["brim_width"], "3");
        // …Vase / Décoration / Prototype / Figurine articulée: none.
        for pt in [Vase, Decoration, PrototypeRapide, FigurineArticulee] {
            let (_, v) = build_one(pt, 0.4, true);
            assert_eq!(v["brim_type"], "no_brim", "{pt:?} should NOT brim");
            assert!(v.get("brim_width").is_none(), "{pt:?} has no brim_width");
        }
        // Auto-support on overhang-prone intents only, with the TYPE matched to
        // the geometry: organic tree for the curvy intents, normal(snug) for the
        // flat mechanical one. Figurine uses a LOW 30° threshold (fewer supports,
        // only pronounced overhangs); Jouet / Pièce mécanique keep 45°.
        for pt in [Figurine, Jouet, PieceMecanique] {
            let (_, v) = build_one(pt, 0.4, true);
            assert_eq!(v["enable_support"], "1", "{pt:?} should auto-support");
        }
        assert_eq!(build_one(Figurine, 0.4, true).1["support_threshold_angle"], "30");
        assert_eq!(build_one(Jouet, 0.4, true).1["support_threshold_angle"], "45");
        assert_eq!(build_one(PieceMecanique, 0.4, true).1["support_threshold_angle"], "45");
        for pt in [Figurine, Jouet] {
            let (_, v) = build_one(pt, 0.4, true);
            assert_eq!(v["support_type"], "tree(auto)", "{pt:?} organic tree");
            assert_eq!(v["support_style"], "organic", "{pt:?} organic style");
        }
        let (_, mech) = build_one(PieceMecanique, 0.4, true);
        assert_eq!(mech["support_type"], "normal(auto)");
        assert_eq!(mech["support_style"], "snug");
        // …never on Vase (spiral forbids it), Prototype, or flat intents.
        for pt in [Vase, PrototypeRapide, ObjetDuQuotidien, Decoration] {
            let (_, v) = build_one(pt, 0.4, true);
            assert!(v.get("enable_support").is_none(), "{pt:?} no auto-support");
        }
        // Articulated figurines: support EXPLICITLY off (would fuse joints) + no brim.
        let (_, art) = build_one(FigurineArticulee, 0.4, true);
        assert_eq!(art["enable_support"], "0", "articulée: support forced off");
        assert_eq!(art["brim_type"], "no_brim", "articulée: no brim");
    }

    #[test]
    fn library_has_32_unique_named_profiles() {
        let lib = build_library(true);
        assert_eq!(lib.len(), 32);
        let mut names: Vec<&str> = lib.iter().map(|(n, _)| n.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 32, "profile names must be unique");
    }

    #[test]
    fn every_profile_is_registrable_and_targets_its_nozzle() {
        for (name, v) in build_library(true) {
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
        let (_, vase) = build_one(ProjectType::Vase, 0.4, true);
        assert_eq!(vase["spiral_mode"], "1");
        assert_eq!(vase["wall_loops"], "1");
        assert_eq!(vase["sparse_infill_density"], "0%");

        // Figurine: tightest cornering (lowest accel & jerk of the set).
        let fig = ProjectType::Figurine.params_at(0.4);
        let proto = ProjectType::PrototypeRapide.params_at(0.4);
        assert!(fig.acceleration < proto.acceleration);
        assert!(fig.jerk < proto.jerk);

        // Décoration enables ironing; Pièce mécanique stacks walls.
        let (_, deco) = build_one(ProjectType::Decoration, 0.4, true);
        assert_eq!(deco["ironing_type"], "topmost");
        assert_eq!(ProjectType::PieceMecanique.params_at(0.4).wall_loops, 5);
    }

    #[test]
    fn jerk_never_exceeds_machine_max() {
        use ProjectType::*;
        // The U1 caps X/Y jerk at 9 mm/s. PrototypeRapide's reference jerk (12)
        // and EVERY profile's travel jerk (1.5× print jerk) would otherwise
        // exceed it and make the slicer warn + silently auto-cap. All emitted
        // jerk keys must be clamped to the ceiling (and never below the 1 floor).
        const JERK_KEYS: &[&str] = &[
            "default_jerk", "outer_wall_jerk", "inner_wall_jerk",
            "top_surface_jerk", "travel_jerk",
        ];
        for pt in ProjectType::all() {
            let (name, v) = build_one(pt, 0.4, true);
            for k in JERK_KEYS {
                let j: f64 = v[*k].as_str().unwrap().parse().unwrap();
                assert!(j <= 9.0, "{name}: {k} = {j} exceeds U1 machine_max_jerk 9");
                assert!(j >= 1.0, "{name}: {k} = {j} below the 1 floor");
            }
        }
        // PrototypeRapide is the binding case (ref jerk 12, travel 18): both must
        // be pulled down to the 9 ceiling, not left at 12/18.
        let (_, proto) = build_one(PrototypeRapide, 0.4, true);
        assert_eq!(proto["default_jerk"], "9");
        assert_eq!(proto["travel_jerk"], "9");
        // The clamp is a CEILING, not a rewrite: a printer with no jerk limit
        // (max_jerk == 0) keeps the richer reference-derived values.
        let unbounded = PrinterSpec {
            printer_name: "Unbounded (0.4 nozzle)".into(),
            base_process: "base".into(),
            nozzle: 0.4,
            max_layer_height: 0.30,
            architecture: Architecture::Single,
            max_jerk: 0.0,
        };
        let (_, vp) = build_one_for(PrototypeRapide, &unbounded, false);
        assert_eq!(vp["default_jerk"], "12");
        assert_eq!(vp["travel_jerk"], "18");
    }

    #[test]
    fn fork_features_on_every_profile() {
        for (name, v) in build_library(true) {
            assert_eq!(v["filament_economy_enable"], "1", "{name}");
            assert_eq!(v["mixed_filament_region_collapse"], "1", "{name}");
        }
    }

    #[test]
    fn universal_economy_on_every_profile() {
        // The cross-slicer, preview-visible economy (v0.5.0). These must be set
        // on every generated profile and must NOT route purge into the object
        // body (flush_into_objects) — that would bleed colour onto the surface.
        for (name, v) in build_library(true) {
            assert_eq!(v["flush_into_infill"], "1", "{name}");
            assert_eq!(v["flush_into_support"], "1", "{name}");
            assert_eq!(v["wipe_tower_no_sparse_layers"], "1", "{name}");
            // prime_volume is THE tower lever on tool-changer / non-purge-in-
            // tower printers; flush_multiplier is the AMS lever. We set both.
            // prime_volume now scales with nozzle (~38×), so assert a sane range.
            let pv: i64 = v["prime_volume"].as_str().unwrap().parse().unwrap();
            assert!((4..=40).contains(&pv), "{name}: prime_volume {pv} out of range");
            assert_eq!(v["flush_multiplier"], "0.2", "{name}");
            assert_eq!(v["prime_tower_width"], "30", "{name}");
            assert!(
                v.get("flush_into_objects").is_none(),
                "{name}: flush_into_objects must stay unset (colour-bleed risk)"
            );
        }
    }
}
