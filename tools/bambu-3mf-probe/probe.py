#!/usr/bin/env python3
"""
Bambu .3mf -> Snapmaker U1 conversion probe.

Mirrors the BambuConvert C++ algorithm (sRGB->linear->Lab, CIEDE2000,
top-N selection + (pair, ratio) FullSpectrum overflow synthesis) so we
can dry-run a real Bambu .3mf and see what convert_filament_list()
would produce without needing a slicer build.

Usage:
    python probe.py <path-to.3mf> [--extra "#RRGGBB[:used_mm[:type]]" ...]

The --extra flags inject synthetic overflow filaments on top of the
ones found in the file, useful for stress-testing the overflow path
on real Bambu palettes.
"""
import argparse
import json
import math
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path


# --- color math ------------------------------------------------------------

def srgb_to_linear(s):
    s = max(0.0, min(1.0, s))
    return s / 12.92 if s <= 0.04045 else ((s + 0.055) / 1.055) ** 2.4


def linear_to_srgb(l):
    l = max(0.0, min(1.0, l))
    return l * 12.92 if l <= 0.0031308 else 1.055 * (l ** (1.0 / 2.4)) - 0.055


def parse_hex(h):
    if not h or h[0] != '#' or len(h) not in (7, 9):
        return (0.0, 0.0, 0.0)
    rs, gs, bs = (int(h[1:3], 16) / 255.0,
                  int(h[3:5], 16) / 255.0,
                  int(h[5:7], 16) / 255.0)
    return (srgb_to_linear(rs), srgb_to_linear(gs), srgb_to_linear(bs))


def format_hex(c):
    def b(v): return max(0, min(255, round(linear_to_srgb(v) * 255)))
    return "#%02X%02X%02X" % (b(c[0]), b(c[1]), b(c[2]))


def rgb_to_xyz(c):
    r, g, b = c[0] * 100, c[1] * 100, c[2] * 100
    return (r * 0.4124564 + g * 0.3575761 + b * 0.1804375,
            r * 0.2126729 + g * 0.7151522 + b * 0.0721750,
            r * 0.0193339 + g * 0.1191920 + b * 0.9503041)


XN, YN, ZN = 95.047, 100.0, 108.883


def _f(t):
    d = 6.0 / 29.0
    return t ** (1.0 / 3.0) if t > d * d * d else t / (3 * d * d) + 4.0 / 29.0


def rgb_to_lab(c):
    x, y, z = rgb_to_xyz(c)
    fx, fy, fz = _f(x / XN), _f(y / YN), _f(z / ZN)
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))


def ciede2000(lab1, lab2):
    L1, a1, b1 = lab1
    L2, a2, b2 = lab2
    C1 = math.hypot(a1, b1)
    C2 = math.hypot(a2, b2)
    Cb = 0.5 * (C1 + C2)
    Cb7 = Cb ** 7
    G = 0.5 * (1.0 - math.sqrt(Cb7 / (Cb7 + 25.0 ** 7)))
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = math.hypot(a1p, b1), math.hypot(a2p, b2)

    def hp(b, a):
        if b == 0 and a == 0:
            return 0
        h = math.degrees(math.atan2(b, a))
        return h + 360 if h < 0 else h

    h1p, h2p = hp(b1, a1p), hp(b2, a2p)
    dLp = L2 - L1
    dCp = C2p - C1p
    if C1p * C2p == 0:
        dhp = 0
    else:
        diff = h2p - h1p
        dhp = diff - 360 if diff > 180 else diff + 360 if diff < -180 else diff
    dHp = 2 * math.sqrt(C1p * C2p) * math.sin(math.radians(dhp) / 2)
    Lbp, Cbp = 0.5 * (L1 + L2), 0.5 * (C1p + C2p)
    if C1p * C2p == 0:
        Hbp = h1p + h2p
    else:
        s, d = h1p + h2p, abs(h1p - h2p)
        Hbp = 0.5 * s if d <= 180 else 0.5 * (s + 360) if s < 360 else 0.5 * (s - 360)
    T = (1 - 0.17 * math.cos(math.radians(Hbp - 30))
            + 0.24 * math.cos(math.radians(2 * Hbp))
            + 0.32 * math.cos(math.radians(3 * Hbp + 6))
            - 0.20 * math.cos(math.radians(4 * Hbp - 63)))
    dTh = 30 * math.exp(-((Hbp - 275) / 25) ** 2)
    Rc = 2 * math.sqrt(Cbp ** 7 / (Cbp ** 7 + 25.0 ** 7))
    Sl = 1 + (0.015 * (Lbp - 50) ** 2) / math.sqrt(20 + (Lbp - 50) ** 2)
    Sc, Sh = 1 + 0.045 * Cbp, 1 + 0.015 * Cbp * T
    Rt = -math.sin(math.radians(2 * dTh)) * Rc
    tL, tC, tH = dLp / Sl, dCp / Sc, dHp / Sh
    return math.sqrt(tL ** 2 + tC ** 2 + tH ** 2 + Rt * tC * tH)


def mix(a, b, ratio_a):
    ratio_a = max(0.0, min(1.0, ratio_a))
    rb = 1.0 - ratio_a
    return (a[0] * ratio_a + b[0] * rb,
            a[1] * ratio_a + b[1] * rb,
            a[2] * ratio_a + b[2] * rb)


MIXING_RATIOS = [i * 0.05 for i in range(1, 20)]  # 0.05..0.95 step 0.05


# --- assignment ------------------------------------------------------------

def _best_mix_for_target(target_lab, phys_rgb, phys_lab):
    """Try every (a, b, ratio) combination, return the best recipe."""
    best = {"delta_e": float("inf")}
    n = len(phys_rgb)
    for a in range(n):
        for b in range(n):
            if a == b:
                continue
            for r in MIXING_RATIOS:
                mixed = mix(phys_rgb[a], phys_rgb[b], r)
                de = ciede2000(rgb_to_lab(mixed), target_lab)
                if de < best["delta_e"]:
                    best.update({
                        "physical_a": a, "physical_b": b, "ratio_a": r,
                        "achieved_hex": format_hex(mixed), "delta_e": de,
                    })
    return best


def _virtuals_for(inputs, physicals):
    """Given a fixed physical set, synthesise virtuals for every input
    not in `physicals`. Returns (virtual_list, sum_delta_e)."""
    phys_rgb = [parse_hex(inputs[i]["color_hex"]) for i in physicals]
    phys_lab = [rgb_to_lab(c) for c in phys_rgb]
    virtuals = []
    total_de = 0.0
    phys_set = set(physicals)
    for k, fil in enumerate(inputs):
        if k in phys_set:
            continue
        target_lab = rgb_to_lab(parse_hex(fil["color_hex"]))
        best = _best_mix_for_target(target_lab, phys_rgb, phys_lab)
        best.update({"input_idx": k, "target_hex": fil["color_hex"]})
        virtuals.append(best)
        total_de += best["delta_e"]
    return virtuals, total_de


def convert_filament_list(inputs, cap=4, strategy="usage"):
    """inputs: list of dicts {color_hex, used_mm, type}.

    strategy:
        "usage"     — pick the top `cap` filaments by used_mm.
        "chromatic" — exhaustive search minimising sum overflow deltaE
                      (unweighted; pure perceptual quality).
        "balanced"  — exhaustive search minimising sum
                      (deltaE * used_mm). High-deltaE overflows on
                      barely-used colors cost less than small drift on
                      heavy filaments.
    """
    from itertools import combinations

    if not inputs:
        return {"physical_indices": [], "virtuals": [], "strategy": strategy}

    n_phys = min(cap, len(inputs))

    if len(inputs) <= cap:
        physicals = list(range(len(inputs)))
        return {"physical_indices": physicals, "virtuals": [],
                "strategy": strategy, "total_overflow_delta_e": 0.0}

    if strategy == "usage":
        order = sorted(range(len(inputs)),
                       key=lambda i: (-inputs[i]["used_mm"], i))
        physicals = order[:n_phys]
        virtuals, total_de = _virtuals_for(inputs, physicals)
        return {"physical_indices": physicals, "virtuals": virtuals,
                "strategy": "usage", "total_overflow_delta_e": total_de}

    if strategy in ("chromatic", "balanced"):
        best_phys = None
        best_virts = None
        best_score = float("inf")
        best_total_de = float("inf")
        candidates = 0
        phys_set = set()
        for combo in combinations(range(len(inputs)), n_phys):
            virts, total_de = _virtuals_for(inputs, combo)
            if strategy == "chromatic":
                score = total_de
            else:  # balanced
                phys_set = set(combo)
                score = sum(
                    v["delta_e"] * max(1.0, inputs[v["input_idx"]]["used_mm"])
                    for v in virts
                )
            candidates += 1
            if score < best_score:
                best_score = score
                best_total_de = total_de
                best_phys = list(combo)
                best_virts = virts
        return {"physical_indices": best_phys, "virtuals": best_virts,
                "strategy": strategy,
                "candidates_considered": candidates,
                "total_overflow_delta_e": best_total_de,
                "total_weighted_delta_e": best_score if strategy == "balanced"
                                          else None}

    raise ValueError(f"unknown strategy: {strategy}")


# --- 3mf parsing -----------------------------------------------------------

def read_bambu_3mf(path):
    with zipfile.ZipFile(path) as z:
        with z.open("Metadata/project_settings.config") as f:
            cfg = json.load(f)
        # slice_info.config — XML, holds per-filament real usage (used_m, used_g)
        # populated by the slicer. Optional; older / unsliced files won't have it.
        usage_m = {}
        try:
            with z.open("Metadata/slice_info.config") as f:
                xml = ET.parse(f).getroot()
            for fil in xml.iter("filament"):
                # filament id is 1-based in slice_info; convert to 0-based.
                idx = int(fil.attrib["id"]) - 1
                # Convert m -> mm so it lines up with the algorithm's used_mm.
                usage_m[idx] = float(fil.attrib.get("used_m", "0")) * 1000.0
        except KeyError:
            pass  # no slice info, fall back to flat 100.0 in caller
    colours = cfg.get("filament_colour", [])
    types = cfg.get("filament_type", [])
    return {
        "printer_model": cfg.get("printer_model", "?"),
        "filament_count": len(colours),
        "colours": colours,
        "types": types,
        "settings_id": cfg.get("filament_settings_id", []),
        "ids": cfg.get("filament_ids", []),
        "flush_matrix": cfg.get("flush_volumes_matrix", []),
        "usage_mm": usage_m,
    }


# --- main ------------------------------------------------------------------

def main():
    p = argparse.ArgumentParser()
    p.add_argument("file")
    p.add_argument("--extra", action="append", default=[],
                   help="extra synthetic filament '#RRGGBB[:used_mm[:type]]'")
    p.add_argument("--strategy", default="usage",
                   choices=["usage", "chromatic", "balanced", "all"],
                   help="physical selection strategy (default: usage)")
    args = p.parse_args()

    info = read_bambu_3mf(args.file)
    has_usage = bool(info["usage_mm"])
    print(f"file:           {args.file}")
    print(f"printer_model:  {info['printer_model']}")
    print(f"filament count: {info['filament_count']}"
          + ("  (real usage from slice_info.config)" if has_usage
             else "  (no slice_info, using flat usage = 100mm)"))
    for i, (c, t) in enumerate(zip(info["colours"], info["types"])):
        if has_usage:
            u = info["usage_mm"].get(i, 0.0)
            print(f"  [{i}] {c}  {t:5s}  used = {u/1000:6.2f} m")
        else:
            print(f"  [{i}] {c}  {t}")
    print()

    inputs = []
    for i, (c, t) in enumerate(zip(info["colours"], info["types"])):
        used = info["usage_mm"].get(i, 100.0) if has_usage else 100.0
        inputs.append({"color_hex": c, "used_mm": used, "type": t or "PLA"})

    for x in args.extra:
        parts = x.split(":")
        color = parts[0]
        used = float(parts[1]) if len(parts) > 1 else 100.0
        ftype = parts[2] if len(parts) > 2 else "PLA"
        inputs.append({"color_hex": color, "used_mm": used, "type": ftype})

    if args.extra:
        print(f"+ {len(args.extra)} synthetic extras injected for overflow test")
        for x in args.extra:
            print(f"  {x}")
        print()

    def print_result(res):
        tag = res["strategy"]
        if "candidates_considered" in res:
            tag += f" ({res['candidates_considered']} candidates)"
        print(f"CONVERT_FILAMENT_LIST [{tag}]:")
        print(f"  physical filaments ({len(res['physical_indices'])}):")
        for slot, idx in enumerate(res["physical_indices"]):
            f = inputs[idx]
            print(f"    slot {slot} <- input[{idx}]  {f['color_hex']}  {f['type']}")
        if not res["virtuals"]:
            print("  virtual filaments: (none, no overflow)")
        else:
            print(f"  virtual filaments ({len(res['virtuals'])}):")
            for v in sorted(res["virtuals"], key=lambda v: v["delta_e"]):
                print(f"    input[{v['input_idx']}] target={v['target_hex']}  ->  "
                      f"mix(slot {v['physical_a']}, slot {v['physical_b']}, "
                      f"ratio_a={v['ratio_a']:.3f})  "
                      f"achieved={v['achieved_hex']}  deltaE={v['delta_e']:.2f}")
        if "total_overflow_delta_e" in res:
            print(f"  sum overflow deltaE = {res['total_overflow_delta_e']:.2f}")

    strategies = (["usage", "chromatic", "balanced"]
                  if args.strategy == "all" else [args.strategy])
    for i, strat in enumerate(strategies):
        if i:
            print()
        print_result(convert_filament_list(inputs, strategy=strat))


if __name__ == "__main__":
    main()
