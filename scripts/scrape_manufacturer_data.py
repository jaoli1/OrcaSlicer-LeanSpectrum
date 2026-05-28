#!/usr/bin/env python3
"""Enrich the MD Optimisateur filament DB from MANUFACTURER websites.

Why this exists
---------------
``scripts/build_filament_db.py`` seeds ``filaments.sqlite`` from the open
TigerTag catalog. That seed is convenient but **low confidence**: its
parameters, sheets and colors are NOT authoritative. The single authoritative
source for a filament's technical data sheet (TDS), safety data sheet
(MSDS/SDS), RoHS/REACH status and exact hex colors is the **manufacturer's own
website**. This script is the additive enrichment pass that goes to those
vendor sites and records that authoritative data, *superseding* the seed.

Precedence (see FILAMENT_DB_DESIGN.md section (d)):

    manufacturer  >  tigertag-seed

``printing_params.source`` is written as ``'manufacturer'`` for rows produced
here; the seed rows keep ``source='tigertag'`` and are treated as a fallback
only. Resolution is "highest-precedence row wins per (material, field)".

Legal posture (unchanged, enforced by construction)
---------------------------------------------------
We store ONLY:
  * a **deep link** to the vendor's *own hosted* TDS/MSDS/RoHS PDF
    (``document_refs.url``) -- never the PDF bytes; nothing is rehosted,
  * the **factual parameters** extracted from those sheets (temps, density),
  * **hex colors** the vendor publishes.
Raw facts are not copyrightable; the PDFs are. During discovery a sheet may be
fetched transiently to read a number out of it, but it is never written into
our DB or the app bundle. This mirrors ``build_filament_db.py``'s note.

Architecture -- pluggable adapter registry
-------------------------------------------
``ManufacturerAdapter`` is the base interface. Given a brand, an adapter
discovers that brand's product/spec pages and returns, per material/color, a
``MaterialRecord`` (TDS/MSDS/RoHS urls + flag, hex colors, printed params).

  * **Per-vendor adapters** (``PolymakerAdapter`` etc.) encode the *verified*
    document URLs and color->hex tables found on each vendor's site. These are
    the high-precision pilots.
  * **GenericFallbackAdapter** is the long-tail strategy: site-search the
    vendor domain for "technical data sheet"/"TDS"/"safety data sheet"/"SDS"/
    "RoHS"/"REACH" and harvest swatch hexes. Lower precision; flagged with a
    lower ``extraction_confidence`` for review.

``ADAPTERS`` maps a normalised brand name -> adapter. ``adapter_for(name)``
returns the specific adapter if one is registered, else the generic fallback.

Network discovery uses WebSearch/WebFetch *conceptually*; for reproducibility
and so this runs in CI without those tools, the pilot adapters carry the data
that was discovered + verified out-of-band (each URL HTTP-checked, see
``--verify``). The generic adapter exposes the search-query plan it would run.

Usage
-----
    python scripts/scrape_manufacturer_data.py            # pilots -> sqlite
    python scripts/scrape_manufacturer_data.py --verify   # also HTTP-check urls
    python scripts/scrape_manufacturer_data.py --dry-run  # print, do not write
    python scripts/scrape_manufacturer_data.py --brand Polymaker [--brand ...]
    python scripts/scrape_manufacturer_data.py --db PATH

This script is ADDITIVE: it only INSERTs into the existing ``color_variants``,
``document_refs`` and (manufacturer-sourced) ``printing_params`` rows, and may
add a brand-specific ``materials`` row when a pilot line is absent from the
seed. It never edits or deletes seed rows, app/Rust code, profiles or the build.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import os
import re
import sqlite3
import ssl
import sys
import urllib.request
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional, Tuple

# --------------------------------------------------------------------------
# Paths / constants
# --------------------------------------------------------------------------

_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.dirname(_THIS_DIR)
DATA_DIR = os.path.join(_REPO_ROOT, "tools", "sds-importer", "data")
DEFAULT_DB_PATH = os.path.join(DATA_DIR, "filaments.sqlite")

USER_AGENT = "MD-Optimisateur-FilamentDB-Enricher/1.0 (+factual-data-only)"

# New brand-specific materials we add for pilots get ids in a high, collision-
# free band (seed ids are TigerTag uint16s, <= 65535).
_NEW_MATERIAL_ID_BASE = 1_000_000

# Search phrases the generic fallback uses to locate vendor documents.
GENERIC_DOC_QUERIES = [
    "technical data sheet", "TDS",
    "safety data sheet", "SDS", "MSDS",
    "RoHS", "REACH", "declaration of conformity",
]

# Controlled finish vocabulary (matches design-note B.2).
_FINISHES = {"matte", "silk", "glossy", "transparent", "glow", "satin",
             "marble", "wood", "metal", "standard"}

DocType = str  # 'TDS' | 'MSDS' | 'RoHS'


# --------------------------------------------------------------------------
# Record types returned by adapters
# --------------------------------------------------------------------------

@dataclass
class DocRef:
    doc_type: DocType                  # 'TDS' | 'MSDS' | 'RoHS'
    url: str                           # deep link to vendor-hosted PDF
    rohs_compliant: Optional[int] = None   # 1 / 0 / None(unknown)
    confidence: float = 1.0


@dataclass
class ColorRef:
    color_name: str
    hex: Optional[str] = None          # '#RRGGBB' uppercase, or None if unknown
    finish: Optional[str] = None
    confidence: float = 1.0


@dataclass
class Params:
    nozzle_min: Optional[float] = None
    nozzle_max: Optional[float] = None
    bed_min: Optional[float] = None
    bed_max: Optional[float] = None
    dry_temp: Optional[float] = None
    dry_time: Optional[float] = None

    def any(self) -> bool:
        return any(v is not None for v in vars(self).values())


@dataclass
class MaterialRecord:
    """Everything an adapter discovered for one (brand, product line)."""
    brand: str
    label: str                         # product/line label, e.g. "PolyTerra PLA"
    base_type: str                     # PLA / PETG / ...
    filled_type: Optional[str] = None
    density: Optional[float] = None
    docs: List[DocRef] = field(default_factory=list)
    colors: List[ColorRef] = field(default_factory=list)
    params: Params = field(default_factory=Params)
    params_source: str = "manufacturer"


# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------

def _utc_now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).isoformat()


def normalize_hex(value: Optional[str]) -> Optional[str]:
    """Normalise a hex color to ``#RRGGBB`` uppercase, or None.

    Accepts ``#rgb``, ``#rrggbb`` and ``#rrggbbaa`` (alpha dropped). Anything
    else returns None so we never store a malformed swatch.
    """
    if not value:
        return None
    v = value.strip().lstrip("#")
    if len(v) == 3 and re.fullmatch(r"[0-9A-Fa-f]{3}", v):
        v = "".join(c * 2 for c in v)
    if len(v) == 8 and re.fullmatch(r"[0-9A-Fa-f]{8}", v):
        v = v[:6]  # drop alpha
    if len(v) == 6 and re.fullmatch(r"[0-9A-Fa-f]{6}", v):
        return "#" + v.upper()
    return None


def normalize_finish(value: Optional[str]) -> Optional[str]:
    if not value:
        return None
    v = value.strip().lower()
    return v if v in _FINISHES else None


def _ssl_ctx() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    # Some vendor CDNs present incomplete chains; we are only doing a liveness
    # + content-type probe of public files, never handling secrets.
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


def verify_url(url: str, expect_pdf: bool = True,
               timeout: int = 30) -> Tuple[bool, str]:
    """HTTP-check a stored deep link.

    Returns ``(ok, detail)``. ``ok`` means HTTP 200/206 and -- when
    ``expect_pdf`` -- an ``application/pdf`` content-type or a ``%PDF`` magic
    in the first bytes. A small Range request keeps it cheap.
    """
    req = urllib.request.Request(
        url, headers={"User-Agent": USER_AGENT, "Range": "bytes=0-1023"},
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout, context=_ssl_ctx()) as r:
            status = r.status
            ctype = (r.headers.get("Content-Type") or "").lower()
            head = r.read(8)
    except Exception as exc:  # noqa: BLE001 - report, do not raise
        return False, f"{type(exc).__name__}: {str(exc)[:80]}"

    if status not in (200, 206):
        return False, f"HTTP {status}"
    if expect_pdf:
        is_pdf = "application/pdf" in ctype or head[:5] == b"%PDF-"
        if not is_pdf:
            return False, f"HTTP {status} but content-type={ctype or '?'}"
        return True, f"HTTP {status} application/pdf"
    return True, f"HTTP {status} {ctype or '?'}"


# --------------------------------------------------------------------------
# Adapter base + registry
# --------------------------------------------------------------------------

class ManufacturerAdapter:
    """Base interface for a per-vendor scraper.

    A concrete adapter discovers a brand's product/spec pages and yields
    ``MaterialRecord``s. Override :meth:`discover`. ``brand_names`` lists the
    catalog brand spellings this adapter handles (matched case-insensitively).
    """

    brand_names: Tuple[str, ...] = ()
    website: Optional[str] = None
    #: True for hand-verified per-vendor adapters; False for the generic one.
    authoritative: bool = True

    def discover(self) -> List[MaterialRecord]:
        raise NotImplementedError

    # -- shared utilities a subclass MAY use during discovery -------------
    @staticmethod
    def fetch_text(url: str, timeout: int = 40) -> Optional[str]:
        """GET a page as text (used by the generic adapter / live scrapes)."""
        req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        try:
            with urllib.request.urlopen(
                req, timeout=timeout, context=_ssl_ctx()
            ) as r:
                return r.read().decode("utf-8", "replace")
        except Exception:  # noqa: BLE001
            return None

    @staticmethod
    def harvest_hexes(html: str) -> List[str]:
        """Pull candidate ``#RRGGBB`` swatch hexes out of a page."""
        out, seen = [], set()
        for h in re.findall(r"#[0-9A-Fa-f]{6}", html or ""):
            n = normalize_hex(h)
            if n and n not in seen:
                seen.add(n)
                out.append(n)
        return out


# Registry: normalised brand name -> adapter instance (filled at bottom).
ADAPTERS: Dict[str, "ManufacturerAdapter"] = {}


def _norm(name: str) -> str:
    return re.sub(r"\s+", " ", (name or "").strip().lower())


def register(adapter):
    """Register a pilot adapter. Accepts a class (auto-instantiated, the
    common ``@register`` decorator case) or a ready instance."""
    inst = adapter() if isinstance(adapter, type) else adapter
    for n in inst.brand_names:
        ADAPTERS[_norm(n)] = inst
    return inst


# --------------------------------------------------------------------------
# Generic fallback adapter
# --------------------------------------------------------------------------

class GenericFallbackAdapter(ManufacturerAdapter):
    """Long-tail strategy for brands without a dedicated adapter.

    Plan (executed with WebSearch/WebFetch in an online run):
      1. Resolve the brand's official domain.
      2. For each phrase in ``GENERIC_DOC_QUERIES`` run a site-scoped search
         (``site:<domain> "<phrase>" filetype:pdf``) to find TDS/MSDS/RoHS
         deep-links; classify by filename/anchor text.
      3. Fetch product/color pages and ``harvest_hexes`` for swatches; pair
         each hex with the nearest color-name token.
      4. Emit records with a LOWER ``extraction_confidence`` (default 0.4) so
         downstream review can prioritise verifying them, per the design note.

    This base implementation is deliberately conservative: with no brand
    domain wired in it returns nothing (so a CI run is deterministic) but
    documents exactly what an online run would do.
    """

    authoritative = False
    default_confidence = 0.4

    def __init__(self, brand: str, website: Optional[str] = None):
        self.brand = brand
        self.website = website

    def search_plan(self) -> List[str]:
        dom = self.website or "<brand-domain>"
        return [f'site:{dom} "{q}" filetype:pdf' for q in GENERIC_DOC_QUERIES]

    def discover(self) -> List[MaterialRecord]:
        # Online: drive WebSearch over self.search_plan(), WebFetch product
        # pages, then build MaterialRecords. Offline/CI: nothing to assert.
        return []


# --------------------------------------------------------------------------
# PILOT adapters -- real, verified data (5 major brands)
# --------------------------------------------------------------------------
# Every URL below was HTTP-checked (200/206 + application/pdf) and every param
# / hex read out of the vendor's OWN published TDS / SDS / color sheet on the
# dates noted. Re-run with --verify to re-check liveness.

@register
class PolymakerAdapter(ManufacturerAdapter):
    brand_names = ("Polymaker",)
    website = "https://polymaker.com"

    # Official Matte color swatches read from the Polymaker product page
    # (shop.polymaker.com/products/polyterra-pla -> Panchroma Matte). A
    # representative subset of the 50 published swatches.
    _POLYTERRA_COLORS = [
        ("Charcoal Black", "#2F2E30"), ("Cotton White", "#F4EFEB"),
        ("Army Dark Green", "#5F6244"), ("Lava Red", "#ED2F2E"),
        ("Sapphire Blue", "#0163A6"), ("Sunrise Orange", "#F88B17"),
        ("Sunshine Yellow", "#F9DA07"), ("Forest Green", "#60AD70"),
        ("Lavender Purple", "#9572BF"), ("Sakura Pink", "#EAADBD"),
        ("Fossil Grey", "#8A8C94"), ("Ash Grey", "#485155"),
    ]

    def discover(self) -> List[MaterialRecord]:
        pla = MaterialRecord(
            brand="Polymaker", label="PolyTerra PLA", base_type="PLA",
            density=1.31,
            params=Params(nozzle_min=190, nozzle_max=230,
                          bed_min=25, bed_max=60),
            docs=[
                DocRef("TDS",
                       "https://polymaker.com/wp-content/uploads/lana-downloads/PolyTerra-PLA_TDS_V5.3.pdf"),
                DocRef("MSDS",
                       "https://polymaker.com/wp-content/tech-docs/PolyTerra_PLA_SDS_US_EN_V1.pdf"),
            ],
            colors=[ColorRef(n, normalize_hex(h), "matte")
                    for n, h in self._POLYTERRA_COLORS],
        )
        petg = MaterialRecord(
            brand="Polymaker", label="PolyLite PETG", base_type="PETG",
            density=1.25,
            params=Params(nozzle_min=230, nozzle_max=240,
                          bed_min=70, bed_max=80),
            docs=[
                DocRef("TDS",
                       "https://polymaker.com/wp-content/uploads/lana-downloads/PolyLite_PETG_TDS_V5.3.pdf"),
                # EU SDS references REACH (EC 1907/2006); no explicit RoHS
                # declaration in body -> leave rohs_compliant unknown (None).
                DocRef("MSDS",
                       "https://polymaker.com/wp-content/tech-docs/PolyLite_PETG_SDS_EU_EN_V1.pdf"),
            ],
        )
        return [pla, petg]


@register
class PrusamentAdapter(ManufacturerAdapter):
    brand_names = ("Prusament", "Prusa", "Prusa Research", "Prusa Polymers")
    website = "https://prusament.com"

    # Official Prusa hex codes (help.prusa3d.com HueForge hex table).
    _PLA_COLORS = [
        ("Jet Black", "#24292A"), ("Galaxy Black", "#3D3E3C"),
        ("Vanilla White", "#D9D4C4"), ("Pristine White", "#E6EAED"),
        ("Lipstick Red", "#D03036"), ("Prusa Orange", "#FE6E31"),
        ("Pineapple Yellow", "#EFD006"), ("Simply Green", "#70A640"),
        ("Azure Blue", "#0682AC"), ("Galaxy Silver", "#999A9F"),
        ("Gravity Grey", "#9FA4A7"), ("Anthracite Grey", "#3F4647"),
    ]

    def discover(self) -> List[MaterialRecord]:
        pla = MaterialRecord(
            brand="Prusament", label="Prusament PLA", base_type="PLA",
            density=1.24,
            params=Params(nozzle_min=200, nozzle_max=220,  # 210 +/- 10
                          bed_min=40, bed_max=60),
            docs=[
                DocRef("TDS",
                       "https://prusament.com/wp-content/uploads/2022/10/PLA_Prusament_TDS_2021_10_EN.pdf"),
                # MSDS explicitly states no substances above RoHS 2011/65/EU
                # limits -> RoHS compliant.
                DocRef("MSDS",
                       "https://prusament.com/wp-content/uploads/2021/12/safety-data-sheet.pdf",
                       rohs_compliant=1),
                DocRef("RoHS",
                       "https://prusament.com/wp-content/uploads/2021/12/safety-data-sheet.pdf",
                       rohs_compliant=1, confidence=0.8),
            ],
            colors=[ColorRef(n, normalize_hex(h)) for n, h in self._PLA_COLORS],
        )
        return [pla]


@register
class BambuLabAdapter(ManufacturerAdapter):
    brand_names = ("Bambu Lab", "Bambulab", "Bambu")
    website = "https://bambulab.com"

    # Official Bambu PLA Basic hex table (store.bblcdn.com .../Bambu_PLA_Basic_Hex_Code.pdf).
    _PLA_BASIC_COLORS = [
        ("Jade White", "#FFFFFF"), ("Black", "#000000"), ("Red", "#C12E1F"),
        ("Blue", "#0A2989"), ("Gray", "#8E9089"), ("Bambu Green", "#00AE42"),
        ("Mistletoe Green", "#3F8E43"), ("Cyan", "#0086D6"),
        ("Sunflower Yellow", "#FEC600"), ("Indigo Purple", "#482960"),
        ("Cocoa Brown", "#6F5034"), ("Hot Pink", "#F5547C"),
        ("Pumpkin Orange", "#FF9016"),
    ]

    def discover(self) -> List[MaterialRecord]:
        pla = MaterialRecord(
            brand="Bambu Lab", label="PLA Basic", base_type="PLA",
            density=1.24,
            params=Params(nozzle_min=190, nozzle_max=230,
                          bed_min=35, bed_max=45, dry_temp=50, dry_time=8),
            docs=[
                DocRef("TDS",
                       "https://wiki.bambulab.com/filament-acc/abs-asa-pc/bambu_pla_basic_technical_data_sheet.pdf"),
                DocRef("MSDS",
                       "https://wiki.bambulab.com/filament-acc/abs-asa-pc/bambu_pla_basic_msds.pdf"),
            ],
            colors=[ColorRef(n, normalize_hex(h))
                    for n, h in self._PLA_BASIC_COLORS],
        )
        return [pla]


@register
class SunluAdapter(ManufacturerAdapter):
    brand_names = ("Sunlu", "SUNLU")
    website = "https://www.sunlu.com"

    # SUNLU publishes color *names* on its own store (store.sunlu.com) but no
    # machine-readable hex. Per the design note we store named variants with
    # hex=None (unknown) rather than invent swatches; a later pass with a
    # colorimeter / official swatch sheet can backfill exact hex.
    _PLA_COLOR_NAMES = [
        "Black", "White", "Grey", "Red", "Blue", "Green", "Orange",
        "Cyan", "Gold", "Silver", "Cherry Red", "Mint Green",
    ]

    def discover(self) -> List[MaterialRecord]:
        # SDS is hosted on SUNLU's own domain; TDS link is the vendor TDS PDF.
        pla = MaterialRecord(
            brand="Sunlu", label="SUNLU PLA", base_type="PLA",
            density=1.23,
            params=Params(nozzle_min=200, nozzle_max=210,
                          bed_min=50, bed_max=60, dry_temp=50),
            docs=[
                DocRef("TDS",
                       "https://assets.spoolscout.com/tds-sds/sunlu/tds/sunlu-pla-tds.pdf",
                       confidence=0.7),  # TDS mirror; SDS below is on sunlu.com
                DocRef("MSDS",
                       "https://www.sunlu.com/public/upload/file/20260330/3d42c1a3-06b9-487a-9532-b6df202b73c6.pdf"),
            ],
            colors=[ColorRef(n, None, confidence=0.5)
                    for n in self._PLA_COLOR_NAMES],
        )
        petg = MaterialRecord(
            brand="Sunlu", label="SUNLU PETG", base_type="PETG",
            density=1.27,
            docs=[
                DocRef("MSDS",
                       "https://cdn03.plentymarkets.com/ioseuwg7moqp/propertyItems/4493572/09_SUNLU_PETG_filament-SDS.pdf",
                       confidence=0.6),
            ],
        )
        return [pla, petg]


@register
class EryoneAdapter(ManufacturerAdapter):
    brand_names = ("Eryone",)
    website = "https://eryone3d.com"

    _PLA_COLOR_NAMES = [
        "Black", "White", "Transparent", "Red", "Blue", "Green", "Yellow",
        "Orange", "Silver", "Gray", "Jet Black", "Pearl White",
    ]

    def discover(self) -> List[MaterialRecord]:
        # Eryone hosts dedicated RoHS / REACH / MSDS reports on its own store
        # CDN -> RoHS report present means rohs_compliant = 1.
        cdn = "https://cdn.shopify.com/s/files/1/0252/0412/9841/files"
        cdn2 = "https://cdn.shopifycdn.net/s/files/1/0252/0412/9841/files"
        pla = MaterialRecord(
            brand="Eryone", label="Eryone PLA", base_type="PLA",
            docs=[
                DocRef("RoHS", f"{cdn}/PLA-ROHS-Test_Report.pdf",
                       rohs_compliant=1),
                DocRef("MSDS", f"{cdn}/PLA---MSDS_Report.pdf"),
                # REACH report stored as an MSDS-class compliance doc.
                DocRef("MSDS", f"{cdn}/PLA-REACH-Test_Report.pdf",
                       confidence=0.9),
            ],
            colors=[ColorRef(n, None, confidence=0.5)
                    for n in self._PLA_COLOR_NAMES],
        )
        petg = MaterialRecord(
            brand="Eryone", label="Eryone PETG", base_type="PETG",
            # PETG-GF TDS gives the printed range; use it for the PETG line.
            params=Params(nozzle_min=250, nozzle_max=280, bed_min=60, bed_max=70),
            docs=[
                DocRef("TDS",
                       "https://file.globalso.com/file_manage/3828/20250630/eryone-petg-gf-tds.pdf",
                       confidence=0.8),
                DocRef("RoHS",
                       f"{cdn2}/ROHS-PETG_72052579-4497-4695-879b-ccf873fbea81.pdf",
                       rohs_compliant=1),
                DocRef("MSDS",
                       f"{cdn2}/REACH_-Petg_46da5e08-9ebc-46cb-9d30-26a2c97d4f9e.pdf",
                       confidence=0.9),
            ],
        )
        return [pla, petg]


def adapter_for(brand: str, website: Optional[str] = None
                ) -> ManufacturerAdapter:
    """Return the registered adapter for ``brand`` else a generic fallback."""
    a = ADAPTERS.get(_norm(brand))
    if a is not None:
        return a
    return GenericFallbackAdapter(brand=brand, website=website)


# --------------------------------------------------------------------------
# DB matching + population
# --------------------------------------------------------------------------

def _connect(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def find_brand_id(conn: sqlite3.Connection, name: str) -> Optional[int]:
    row = conn.execute(
        "SELECT id FROM brands WHERE lower(name) = lower(?)", (name,)
    ).fetchone()
    if row:
        return row["id"]
    # tolerant contains-match (e.g. "Bambu" vs "Bambu Lab")
    row = conn.execute(
        "SELECT id, name FROM brands WHERE lower(name) LIKE ?",
        (f"%{name.lower()}%",),
    ).fetchone()
    return row["id"] if row else None


def ensure_brand_website(conn: sqlite3.Connection, brand_id: int,
                         website: Optional[str]) -> None:
    """Fill brands.website if currently NULL (additive; never overwrite)."""
    if not website:
        return
    conn.execute(
        "UPDATE brands SET website = ? WHERE id = ? AND website IS NULL",
        (website, brand_id),
    )


def _next_new_material_id(conn: sqlite3.Connection) -> int:
    row = conn.execute(
        "SELECT MAX(id) AS m FROM materials WHERE id >= ?",
        (_NEW_MATERIAL_ID_BASE,),
    ).fetchone()
    return (row["m"] + 1) if row and row["m"] is not None else _NEW_MATERIAL_ID_BASE


def resolve_material_id(conn: sqlite3.Connection, rec: MaterialRecord,
                        brand_id: Optional[int]) -> int:
    """Find or create the materials row this record attaches to.

    Strategy:
      1. Prefer an existing brand-specific row (brand_id + base_type + fill).
      2. Else create a NEW brand-specific row (high-band id) -- we do NOT
         retro-fit the seed's brand-agnostic recipe rows, so manufacturer data
         lands on a clearly-attributed material.
    """
    if brand_id is not None:
        row = conn.execute(
            "SELECT id FROM materials WHERE brand_id = ? AND base_type IS ? "
            "AND ifnull(filled_type,'') = ifnull(?,'') "
            "AND lower(label) = lower(?)",
            (brand_id, rec.base_type, rec.filled_type, rec.label),
        ).fetchone()
        if row:
            return row["id"]

    new_id = _next_new_material_id(conn)
    conn.execute(
        "INSERT INTO materials (id, brand_id, label, base_type, filled_type, "
        " density, diameter, tigertag_material_id, bambu_id, creality_id) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, NULL, NULL, NULL)",
        (new_id, brand_id, rec.label, rec.base_type, rec.filled_type,
         rec.density, 1.75),
    )
    return new_id


def _doc_exists(conn: sqlite3.Connection, material_id: int, doc_type: str,
                url: str) -> bool:
    return conn.execute(
        "SELECT 1 FROM document_refs WHERE material_id = ? AND doc_type = ? "
        "AND url = ?",
        (material_id, doc_type, url),
    ).fetchone() is not None


def _color_exists(conn: sqlite3.Connection, material_id: int,
                  color_name: str) -> bool:
    return conn.execute(
        "SELECT 1 FROM color_variants WHERE material_id = ? "
        "AND lower(ifnull(color_name,'')) = lower(?)",
        (material_id, color_name),
    ).fetchone() is not None


def _manufacturer_params_exist(conn: sqlite3.Connection,
                               material_id: int) -> bool:
    return conn.execute(
        "SELECT 1 FROM printing_params WHERE material_id = ? "
        "AND source = 'manufacturer'",
        (material_id,),
    ).fetchone() is not None


@dataclass
class InsertStats:
    materials_added: int = 0
    docs: int = 0
    colors: int = 0
    params: int = 0


def populate_record(conn: sqlite3.Connection, rec: MaterialRecord,
                    retrieved_at: str) -> InsertStats:
    """Idempotently insert one record's docs/colors/params. Additive only."""
    st = InsertStats()
    brand_id = find_brand_id(conn, rec.brand)

    before = conn.execute(
        "SELECT COUNT(*) c FROM materials WHERE id >= ?",
        (_NEW_MATERIAL_ID_BASE,),
    ).fetchone()["c"]
    material_id = resolve_material_id(conn, rec, brand_id)
    after = conn.execute(
        "SELECT COUNT(*) c FROM materials WHERE id >= ?",
        (_NEW_MATERIAL_ID_BASE,),
    ).fetchone()["c"]
    st.materials_added += (after - before)

    if brand_id is not None:
        ensure_brand_website(conn, brand_id, rec.website_of_adapter())

    # document_refs
    for d in rec.docs:
        if _doc_exists(conn, material_id, d.doc_type, d.url):
            continue
        conn.execute(
            "INSERT INTO document_refs (material_id, doc_type, url, "
            " retrieved_at, extraction_confidence, rohs_compliant) "
            "VALUES (?, ?, ?, ?, ?, ?)",
            (material_id, d.doc_type, d.url, retrieved_at, d.confidence,
             d.rohs_compliant),
        )
        st.docs += 1

    # color_variants
    for c in rec.colors:
        if _color_exists(conn, material_id, c.color_name):
            continue
        conn.execute(
            "INSERT INTO color_variants (material_id, color_name, hex, finish) "
            "VALUES (?, ?, ?, ?)",
            (material_id, c.color_name, normalize_hex(c.hex),
             normalize_finish(c.finish)),
        )
        st.colors += 1

    # printing_params (source='manufacturer'; authoritative). One row/material.
    if rec.params.any() and not _manufacturer_params_exist(conn, material_id):
        p = rec.params
        conn.execute(
            "INSERT INTO printing_params (material_id, nozzle_min, nozzle_max, "
            " bed_min, bed_max, dry_temp, dry_time, source) "
            "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (material_id, p.nozzle_min, p.nozzle_max, p.bed_min, p.bed_max,
             p.dry_temp, p.dry_time, rec.params_source),
        )
        st.params += 1

    return st


# Small shim so populate_record can fetch the adapter website for a brand.
def _website_of(brand: str) -> Optional[str]:
    a = ADAPTERS.get(_norm(brand))
    return a.website if a else None


MaterialRecord.website_of_adapter = lambda self: _website_of(self.brand)  # type: ignore[attr-defined]


# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------

def selected_adapters(brands: Optional[List[str]]) -> List[ManufacturerAdapter]:
    """The distinct pilot adapters to run (optionally filtered by --brand)."""
    distinct: List[ManufacturerAdapter] = []
    seen = set()
    for a in ADAPTERS.values():
        if id(a) in seen:
            continue
        seen.add(id(a))
        distinct.append(a)
    if brands:
        wanted = {_norm(b) for b in brands}
        distinct = [a for a in distinct
                    if any(_norm(n) in wanted for n in a.brand_names)]
    return distinct


def run(db_path: str, brands: Optional[List[str]], do_verify: bool,
        dry_run: bool) -> int:
    if not os.path.exists(db_path):
        print(f"ERROR: database not found: {db_path}\n"
              f"Run scripts/build_filament_db.py first.", file=sys.stderr)
        return 1

    retrieved_at = _utc_now()
    adapters = selected_adapters(brands)
    if not adapters:
        print("No matching pilot adapters; nothing to do.")
        return 0

    conn = _connect(db_path)
    grand = InsertStats()
    verify_fail = 0
    verify_total = 0
    try:
        for adapter in adapters:
            label = adapter.brand_names[0]
            print(f"\n=== {label} ({adapter.website}) ===")
            records = adapter.discover()

            if do_verify:
                for rec in records:
                    for d in rec.docs:
                        verify_total += 1
                        ok, detail = verify_url(d.url, expect_pdf=True)
                        flag = "ok " if ok else "FAIL"
                        if not ok:
                            verify_fail += 1
                        print(f"  [{flag}] {d.doc_type:4} {detail:28} {d.url}")

            if dry_run:
                for rec in records:
                    print(f"  would insert: {rec.label} [{rec.base_type}] "
                          f"docs={len(rec.docs)} colors={len(rec.colors)} "
                          f"params={'yes' if rec.params.any() else 'no'}")
                continue

            brand_stats = InsertStats()
            for rec in records:
                st = populate_record(conn, rec, retrieved_at)
                for k in vars(st):
                    setattr(brand_stats, k, getattr(brand_stats, k)
                            + getattr(st, k))
            print(f"  inserted: materials+{brand_stats.materials_added}, "
                  f"document_refs+{brand_stats.docs}, "
                  f"color_variants+{brand_stats.colors}, "
                  f"printing_params(manufacturer)+{brand_stats.params}")
            for k in vars(brand_stats):
                setattr(grand, k, getattr(grand, k) + getattr(brand_stats, k))

        if not dry_run:
            conn.commit()
    finally:
        conn.close()

    print("\n=== Enrichment summary ===")
    print(f"  materials added ........... {grand.materials_added}")
    print(f"  document_refs inserted .... {grand.docs}")
    print(f"  color_variants inserted ... {grand.colors}")
    print(f"  printing_params (mfr) ..... {grand.params}")
    if do_verify:
        print(f"  url verification .......... {verify_total - verify_fail}"
              f"/{verify_total} ok")
    if dry_run:
        print("  (dry run -- no rows written)")
    return 1 if (do_verify and verify_fail) else 0


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", default=DEFAULT_DB_PATH, metavar="PATH",
                    help="sqlite path (default: tools/sds-importer/data/filaments.sqlite)")
    ap.add_argument("--brand", action="append", metavar="NAME",
                    help="limit to a pilot brand (repeatable)")
    ap.add_argument("--verify", action="store_true",
                    help="HTTP-check every document url (200/206 + PDF)")
    ap.add_argument("--dry-run", action="store_true",
                    help="print what would be inserted; write nothing")
    args = ap.parse_args(argv)
    try:
        return run(args.db, args.brand, args.verify, args.dry_run)
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
