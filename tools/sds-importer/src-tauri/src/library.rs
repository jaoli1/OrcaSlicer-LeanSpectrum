//! v0.2.0 — Bibliothèque Filament: read-only access to the filament database.
//!
//! The database (`filaments.sqlite`) is the product's core asset. It is built
//! offline from manufacturers' own TDS / SDS / MSDS / RoHS sheets (brands,
//! materials, printing params, colour hex codes, document links), bundled as a
//! snapshot, seeded into the app-data dir on first run, and refreshed from the
//! Maison Drabiec server by [`crate::update`]. This module never writes it — it
//! only reads, so the UI can browse/search materials and the one-click flow can
//! turn a chosen material into an [`ExtractedFilament`] for the profile builder.
//!
//! Schema (see data/build):
//!   brands(id, name, website, …)
//!   materials(id, brand_id, label, base_type, filled_type, density, diameter, …)
//!   printing_params(id, material_id, nozzle_min/max, bed_min/max, dry_temp/time, source)
//!   color_variants(id, material_id, color_name, hex, finish)
//!   document_refs(id, material_id, doc_type, url, …)

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::Serialize;

use crate::{polymer::Polymer, update, Error, ExtractedFilament, Result};

/// One row for the searchable list in the Bibliothèque Filament tab.
#[derive(Debug, Clone, Serialize)]
pub struct FilamentSummary {
    pub id: i64,
    pub brand: String,
    pub label: String,
    pub base_type: String,
    pub filled_type: Option<String>,
    pub density: Option<f64>,
    /// Whether the material has manufacturer printing parameters (nozzle/bed).
    pub has_params: bool,
    pub colors: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColorVariant {
    pub color_name: Option<String>,
    pub hex: Option<String>,
    pub finish: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocRef {
    pub doc_type: Option<String>,
    pub url: String,
}

/// Detail shown when a material is selected: the resolved family + the
/// manufacturer printing window, colours and links to the official sheets.
#[derive(Debug, Clone, Serialize)]
pub struct FilamentDetail {
    pub id: i64,
    pub brand: String,
    pub label: String,
    pub base_type: String,
    pub polymer: String,
    pub density: Option<f64>,
    pub nozzle_min: Option<f64>,
    pub nozzle_max: Option<f64>,
    pub bed_min: Option<f64>,
    pub bed_max: Option<f64>,
    pub dry_temp: Option<f64>,
    pub dry_time: Option<f64>,
    pub colors: Vec<ColorVariant>,
    pub documents: Vec<DocRef>,
}

/// Open the bundled/downloaded database read-only at the canonical location.
fn open() -> Result<Connection> {
    let path = update::db_path()
        .ok_or_else(|| Error::Other("no app-data directory for the filament database".into()))?;
    if !path.exists() {
        return Err(Error::Other(
            "the filament database is not installed yet — click \"Rechercher une mise à jour\" to download it".into(),
        ));
    }
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| Error::Other(format!("open filament database: {e}")))
}

fn db_err<E: std::fmt::Display>(e: E) -> Error {
    Error::Other(format!("filament database: {e}"))
}

/// Distinct list of brand names that have at least one material — feeds the
/// "Marque" selector in the Bibliothèque Filament tab.
pub fn list_brands() -> Result<Vec<String>> {
    list_brands_conn(&open()?)
}

fn list_brands_conn(con: &Connection) -> Result<Vec<String>> {
    let mut stmt = con
        .prepare(
            "SELECT DISTINCT b.name FROM brands b \
             JOIN materials m ON m.brand_id = b.id \
             ORDER BY b.name COLLATE NOCASE",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_err)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(db_err)?);
    }
    Ok(out)
}

/// List materials, optionally filtered by a free-text `query` (brand / label /
/// family) AND/OR an exact `brand` (the selector), newest-brand-first, capped
/// at `limit`. The two filters combine (AND) so the free-text search still
/// works within a chosen brand.
pub fn list(query: Option<String>, brand: Option<String>, limit: i64) -> Result<Vec<FilamentSummary>> {
    list_conn(&open()?, query, brand, limit)
}

fn list_conn(
    con: &Connection,
    query: Option<String>,
    brand: Option<String>,
    limit: i64,
) -> Result<Vec<FilamentSummary>> {
    // NULL `like` => no free-text filter; otherwise a single %term% matched
    // against the three text columns. NULL `brand` => no brand filter; otherwise
    // an exact (parameterised) brand-name equality. Everything is parameterised
    // (no injection).
    let like = query
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));
    let brand = brand
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty());
    let sql = "SELECT m.id, b.name AS brand, m.label, m.base_type, m.filled_type, m.density, \
               EXISTS(SELECT 1 FROM printing_params pp WHERE pp.material_id = m.id) AS has_params, \
               (SELECT COUNT(*) FROM color_variants cv WHERE cv.material_id = m.id) AS colors \
               FROM materials m JOIN brands b ON b.id = m.brand_id \
               WHERE (?1 IS NULL OR b.name LIKE ?1 OR m.label LIKE ?1 OR m.base_type LIKE ?1) \
               AND (?2 IS NULL OR b.name = ?2) \
               ORDER BY b.name COLLATE NOCASE, m.label COLLATE NOCASE \
               LIMIT ?3";
    let mut stmt = con.prepare(sql).map_err(db_err)?;
    let rows = stmt
        .query_map(params![like, brand, limit], |r| {
            Ok(FilamentSummary {
                id: r.get("id")?,
                brand: r.get("brand")?,
                label: r.get("label")?,
                base_type: r.get::<_, Option<String>>("base_type")?.unwrap_or_default(),
                filled_type: r.get("filled_type")?,
                density: r.get("density")?,
                has_params: r.get::<_, i64>("has_params")? != 0,
                colors: r.get("colors")?,
            })
        })
        .map_err(db_err)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(db_err)?);
    }
    Ok(out)
}

/// Full detail for one material (used to preview what will be generated).
pub fn get(id: i64) -> Result<FilamentDetail> {
    get_conn(&open()?, id)
}

fn get_conn(con: &Connection, id: i64) -> Result<FilamentDetail> {
    let (brand, label, base_type, density): (String, String, Option<String>, Option<f64>) = con
        .query_row(
            "SELECT b.name, m.label, m.base_type, m.density \
             FROM materials m JOIN brands b ON b.id = m.brand_id WHERE m.id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| Error::Other(format!("filament {id} not found: {e}")))?;
    let base_type = base_type.unwrap_or_default();
    let polymer = Polymer::from_base_type(&base_type);

    let pp: Option<(Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>, Option<f64>)> =
        con.query_row(
            "SELECT nozzle_min, nozzle_max, bed_min, bed_max, dry_temp, dry_time \
             FROM printing_params WHERE material_id = ?1 \
             ORDER BY (source = 'manufacturer') DESC, id LIMIT 1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .optional()
        .map_err(db_err)?;
    let (nozzle_min, nozzle_max, bed_min, bed_max, dry_temp, dry_time) =
        pp.unwrap_or((None, None, None, None, None, None));

    let mut colors = Vec::new();
    {
        let mut stmt = con
            .prepare("SELECT color_name, hex, finish FROM color_variants WHERE material_id = ?1 ORDER BY id")
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![id], |r| {
                Ok(ColorVariant { color_name: r.get(0)?, hex: r.get(1)?, finish: r.get(2)? })
            })
            .map_err(db_err)?;
        for row in rows {
            colors.push(row.map_err(db_err)?);
        }
    }

    let mut documents = Vec::new();
    {
        let mut stmt = con
            .prepare("SELECT doc_type, url FROM document_refs WHERE material_id = ?1 ORDER BY id")
            .map_err(db_err)?;
        let rows = stmt
            .query_map(params![id], |r| Ok(DocRef { doc_type: r.get(0)?, url: r.get(1)? }))
            .map_err(db_err)?;
        for row in rows {
            documents.push(row.map_err(db_err)?);
        }
    }

    Ok(FilamentDetail {
        id,
        brand,
        label,
        base_type,
        polymer: polymer.as_str().to_string(),
        density,
        nozzle_min,
        nozzle_max,
        bed_min,
        bed_max,
        dry_temp,
        dry_time,
        colors,
        documents,
    })
}

/// Map a database material to the [`ExtractedFilament`] the profile builder
/// consumes. The manufacturer printing window becomes the nozzle/bed ranges,
/// the midpoint becomes the recommended value, and polymer-family defaults
/// backfill anything the sheet did not provide (mirroring the PDF path).
pub fn material_to_extracted(id: i64) -> Result<ExtractedFilament> {
    material_to_extracted_conn(&open()?, id)
}

fn material_to_extracted_conn(con: &Connection, id: i64) -> Result<ExtractedFilament> {
    let (label, brand, website, base_type, density): (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<f64>,
    ) = con
        .query_row(
            "SELECT m.label, b.name, b.website, m.base_type, m.density \
             FROM materials m JOIN brands b ON b.id = m.brand_id WHERE m.id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|e| Error::Other(format!("filament {id} not found: {e}")))?;

    let polymer = Polymer::from_base_type(base_type.as_deref().unwrap_or(""));

    let pp: Option<(Option<f64>, Option<f64>, Option<f64>, Option<f64>)> = con
        .query_row(
            "SELECT nozzle_min, nozzle_max, bed_min, bed_max FROM printing_params \
             WHERE material_id = ?1 ORDER BY (source = 'manufacturer') DESC, id LIMIT 1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(db_err)?;

    let mut ef = ExtractedFilament {
        product_name: Some(label),
        manufacturer: Some(brand),
        manufacturer_url: website,
        polymer: Some(polymer),
        density_g_cm3: density.or_else(|| polymer.default_density_g_cm3()),
        source_files: vec![format!("filament-db://material/{id}")],
        ..Default::default()
    };

    if let Some((nmin, nmax, bmin, bmax)) = pp {
        ef.nozzle_temp_min_c = nmin;
        ef.nozzle_temp_max_c = nmax;
        if let (Some(lo), Some(hi)) = (nmin, nmax) {
            ef.nozzle_temp_recommended_c = Some(((lo + hi) / 2.0).round());
        }
        ef.bed_temp_min_c = bmin;
        ef.bed_temp_max_c = bmax;
        if let (Some(lo), Some(hi)) = (bmin, bmax) {
            ef.bed_temp_recommended_c = Some(((lo + hi) / 2.0).round());
        }
    }

    // Backfill any still-missing nozzle/bed from the polymer family defaults so
    // the generated profile is always usable (flagged in estimated_fields).
    let mut log = Vec::new();
    crate::profile::estimate_missing_temperatures(&mut ef, &mut log);
    Ok(ef)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway in-memory DB with the columns the readers touch + 2 rows.
    fn seed() -> Connection {
        let con = Connection::open_in_memory().unwrap();
        con.execute_batch(
            "CREATE TABLE brands(id INTEGER PRIMARY KEY, name TEXT, website TEXT);
             CREATE TABLE materials(id INTEGER PRIMARY KEY, brand_id INTEGER, label TEXT, base_type TEXT, filled_type TEXT, density REAL, diameter REAL);
             CREATE TABLE printing_params(id INTEGER PRIMARY KEY, material_id INTEGER, nozzle_min REAL, nozzle_max REAL, bed_min REAL, bed_max REAL, dry_temp REAL, dry_time REAL, source TEXT);
             CREATE TABLE color_variants(id INTEGER PRIMARY KEY, material_id INTEGER, color_name TEXT, hex TEXT, finish TEXT);
             CREATE TABLE document_refs(id INTEGER PRIMARY KEY, material_id INTEGER, doc_type TEXT, url TEXT);
             INSERT INTO brands(id,name,website) VALUES (1,'Polymaker','https://polymaker.com'),(2,'eSUN',NULL);
             INSERT INTO materials(id,brand_id,label,base_type,filled_type,density,diameter) VALUES
               (10,1,'PolyTerra PLA','PLA',NULL,1.31,1.75),
               (11,2,'eSUN PETG','PETG',NULL,NULL,1.75);
             INSERT INTO printing_params(material_id,nozzle_min,nozzle_max,bed_min,bed_max,source) VALUES
               (10,190,230,25,60,'manufacturer');
             INSERT INTO color_variants(material_id,color_name,hex,finish) VALUES (10,'Army Green','#4B5320','matte');
             INSERT INTO document_refs(material_id,doc_type,url) VALUES (10,'TDS','https://polymaker.com/tds.pdf');",
        )
        .unwrap();
        con
    }

    #[test]
    fn list_filters_and_orders() {
        let con = seed();
        let all = list_conn(&con, None, None, 100).unwrap();
        assert_eq!(all.len(), 2);
        // Ordered by brand: eSUN before Polymaker.
        assert_eq!(all[0].brand, "eSUN");
        assert_eq!(all[1].brand, "Polymaker");
        assert!(all[1].has_params, "PolyTerra has manufacturer params");
        assert!(!all[0].has_params, "eSUN PETG has none");
        assert_eq!(all[1].colors, 1);

        // Free-text filter matches brand or label or family.
        let petg = list_conn(&con, Some("petg".into()), None, 100).unwrap();
        assert_eq!(petg.len(), 1);
        assert_eq!(petg[0].label, "eSUN PETG");
    }

    #[test]
    fn list_brands_distinct_and_sorted() {
        let con = seed();
        let brands = list_brands_conn(&con).unwrap();
        // Only brands that own at least one material, sorted NOCASE.
        assert_eq!(brands, vec!["eSUN".to_string(), "Polymaker".to_string()]);
    }

    #[test]
    fn list_filters_by_brand_and_combines_with_text() {
        let con = seed();
        // Exact brand filter narrows to that brand only.
        let poly = list_conn(&con, None, Some("Polymaker".into()), 100).unwrap();
        assert_eq!(poly.len(), 1);
        assert_eq!(poly[0].label, "PolyTerra PLA");

        // Brand + free-text that doesn't match within the brand → empty.
        let none = list_conn(&con, Some("PETG".into()), Some("Polymaker".into()), 100).unwrap();
        assert!(none.is_empty());

        // Brand + matching free-text within the brand → the row.
        let some = list_conn(&con, Some("PLA".into()), Some("Polymaker".into()), 100).unwrap();
        assert_eq!(some.len(), 1);

        // Empty-string brand behaves like "no brand filter".
        let all = list_conn(&con, None, Some("".into()), 100).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn material_maps_to_extracted_with_midpoints_and_defaults() {
        let con = seed();
        let ef = material_to_extracted_conn(&con, 10).unwrap();
        assert_eq!(ef.product_name.as_deref(), Some("PolyTerra PLA"));
        assert_eq!(ef.manufacturer.as_deref(), Some("Polymaker"));
        assert_eq!(ef.polymer, Some(Polymer::Pla));
        assert_eq!(ef.density_g_cm3, Some(1.31));
        // nozzle range carried; recommended = rounded midpoint (190+230)/2 = 210.
        assert_eq!(ef.nozzle_temp_min_c, Some(190.0));
        assert_eq!(ef.nozzle_temp_max_c, Some(230.0));
        assert_eq!(ef.nozzle_temp_recommended_c, Some(210.0));
        // bed midpoint (25+60)/2 = 42.5 -> 43 (rounded).
        assert_eq!(ef.bed_temp_recommended_c, Some(43.0));
    }

    #[test]
    fn material_without_params_backfills_density_default() {
        let con = seed();
        // eSUN PETG has no density and no printing params → polymer defaults.
        let ef = material_to_extracted_conn(&con, 11).unwrap();
        assert_eq!(ef.polymer, Some(Polymer::Petg));
        assert_eq!(ef.density_g_cm3, Polymer::Petg.default_density_g_cm3());
        // estimate_missing_temperatures fills the PETG default nozzle range.
        assert!(ef.nozzle_temp_min_c.is_some());
    }

    #[test]
    fn detail_includes_colors_and_documents() {
        let con = seed();
        let d = get_conn(&con, 10).unwrap();
        assert_eq!(d.polymer, "PLA");
        assert_eq!(d.colors.len(), 1);
        assert_eq!(d.colors[0].hex.as_deref(), Some("#4B5320"));
        assert_eq!(d.documents.len(), 1);
        assert_eq!(d.documents[0].doc_type.as_deref(), Some("TDS"));
    }
}
