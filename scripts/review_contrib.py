#!/usr/bin/env python3
"""
Review the community-contribution moderation queue (READ-ONLY).

The Optimisateur's opt-in "share this sheet" feature appends one JSON object
per line (JSONL) to a pending queue on the VPS. This tool pretty-prints each
pending contribution so the owner can eyeball it before accepting it into the
filament database. It NEVER deletes, edits, or sends anything — it only reads
the file and prints to stdout.

Usage:
    python scripts/review_contrib.py [pending.jsonl]

Default path: /var/lib/optimisateur-contrib/pending.jsonl
"""
import argparse
import json
import sys

DEFAULT_PATH = "/var/lib/optimisateur-contrib/pending.jsonl"

# The manufacturer facts we expect (mirrors the client whitelist). Anything
# outside this set is flagged so unexpected keys are visible during review.
KNOWN_KEYS = {
    "brand", "label", "base_type", "density", "diameter",
    "nozzle_min", "nozzle_max", "bed_min", "bed_max",
    "dry_temp", "dry_time", "manufacturer_url", "revision_date",
    "app_version", "submission_id", "colors",
    # server-side annotations on the queued record:
    "ts", "ip_hash",
}


def fmt_range(lo, hi, unit=""):
    """Render a min/max pair compactly, tolerating missing ends."""
    if lo is None and hi is None:
        return "—"
    if lo is not None and hi is not None:
        return f"{lo}–{hi}{unit}"
    return f"{lo if lo is not None else hi}{unit}"


def fmt_colors(colors):
    if not colors:
        return "—"
    out = []
    for c in colors:
        if isinstance(c, dict):
            name = c.get("name", "?")
            hexv = c.get("hex", "")
            out.append(f"{name} ({hexv})" if hexv else str(name))
        else:
            out.append(str(c))
    return ", ".join(out)


def print_record(idx, rec):
    if not isinstance(rec, dict):
        print(f"#{idx}  <not a JSON object: {rec!r}>")
        return

    brand = rec.get("brand", "?")
    label = rec.get("label", "?")
    base_type = rec.get("base_type", "?")
    ts = rec.get("ts", "—")
    ip_hash = rec.get("ip_hash", "—")

    print(f"#{idx}  {brand} — {label}   [{base_type}]")
    print(f"     ts={ts}   ip_hash={ip_hash}")
    print(f"     submission_id={rec.get('submission_id', '—')}   app={rec.get('app_version', '—')}")
    print(f"     density={rec.get('density', '—')}   diameter={rec.get('diameter', '—')}")
    print(f"     nozzle={fmt_range(rec.get('nozzle_min'), rec.get('nozzle_max'), ' C')}"
          f"   bed={fmt_range(rec.get('bed_min'), rec.get('bed_max'), ' C')}")
    print(f"     dry={fmt_range(rec.get('dry_temp'), rec.get('dry_time'))}"
          f"   url={rec.get('manufacturer_url', '—')}")
    print(f"     revision_date={rec.get('revision_date', '—')}")
    print(f"     colors: {fmt_colors(rec.get('colors'))}")

    unexpected = sorted(k for k in rec.keys() if k not in KNOWN_KEYS)
    if unexpected:
        print(f"     ! unexpected keys: {', '.join(unexpected)}")
    print()


def main(argv=None):
    parser = argparse.ArgumentParser(description="Pretty-print the pending contribution queue (read-only).")
    parser.add_argument("path", nargs="?", default=DEFAULT_PATH,
                        help=f"JSONL queue file (default: {DEFAULT_PATH})")
    args = parser.parse_args(argv)

    try:
        with open(args.path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"No pending queue at {args.path} (nothing to review).")
        return 0
    except OSError as e:
        print(f"Could not read {args.path}: {e}", file=sys.stderr)
        return 1

    count = 0
    bad = 0
    for n, line in enumerate(lines, start=1):
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError as e:
            bad += 1
            print(f"#{n}  <invalid JSON: {e}>")
            print()
            continue
        count += 1
        print_record(count, rec)

    print("-" * 60)
    print(f"{count} pending contribution(s) in {args.path}"
          + (f"  ({bad} unparseable line(s))" if bad else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
