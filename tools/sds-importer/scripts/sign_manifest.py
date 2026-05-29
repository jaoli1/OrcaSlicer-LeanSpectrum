#!/usr/bin/env python3
"""Sign a manifest JSON with the ed25519 private key.

Usage
-----
    python sign_manifest.py path/to/manifest-vXYZ.json [--db path/to/filaments.sqlite]

Reads the manifest in place, computes the deterministic signed-payload bytes,
signs them with the local ed25519 private key, and writes the manifest BACK
with a `signature` field appended (and, if --db is provided, a `db_sha256`
field with the local DB's SHA-256).

The signed payload format is intentionally NOT canonical JSON — JSON
canonicalization has too many fiddly cross-implementation edge cases
(whitespace, key order, unicode escapes). Instead we sign a deterministic
byte concatenation with NUL separators:

    SIGNED_PAYLOAD_v1 =
        b"v1" +
        b"\\x00" + utf8(app_version)  +
        b"\\x00" + utf8(db_version)   +
        b"\\x00" + utf8(db_url)       +
        b"\\x00" + utf8(download_url) +
        b"\\x00" + utf8(db_sha256)    +
        b"\\x00" + utf8(notes)

The Rust client builds the exact same bytes from its parsed Manifest struct
and verifies the ed25519 signature with the embedded public key. The leading
"v1" tag is the signed-payload format version — bump if you ever change the
field set / order, and ship a client that handles both.
"""

from __future__ import annotations

import hashlib
import json
import os
import sys
from base64 import b64encode
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


SIGNED_PAYLOAD_VERSION = b"v1"
SIGNED_FIELDS = (
    "app_version",
    "db_version",
    "db_url",
    "download_url",
    "db_sha256",
    "notes",
)


def key_path() -> Path:
    home = os.environ.get("USERPROFILE") or os.path.expanduser("~")
    return Path(home) / ".maison_drabiec" / "manifest_signing.ed25519"


def load_private_key() -> Ed25519PrivateKey:
    kp = key_path()
    if not kp.exists():
        raise SystemExit(
            f"ERROR: signing key not found at {kp}.\n"
            "Run scripts/generate_signing_keypair.py once to create it."
        )
    seed = kp.read_bytes()
    if len(seed) != 32:
        raise SystemExit(
            f"ERROR: {kp} is not a 32-byte raw ed25519 seed (got {len(seed)} bytes)."
        )
    return Ed25519PrivateKey.from_private_bytes(seed)


def build_signed_payload(m: dict) -> bytes:
    """Deterministic byte concatenation — must match `build_signed_payload` in
    src-tauri/src/update.rs exactly, NUL separators included."""
    parts = [SIGNED_PAYLOAD_VERSION]
    for field in SIGNED_FIELDS:
        value = m.get(field, "")
        if not isinstance(value, str):
            raise SystemExit(f"ERROR: manifest field {field!r} must be a string, got {type(value).__name__}")
        parts.append(b"\x00")
        parts.append(value.encode("utf-8"))
    return b"".join(parts)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print("Usage: sign_manifest.py manifest.json [--db filaments.sqlite]", file=sys.stderr)
        return 2

    manifest_path = Path(argv[1])
    if not manifest_path.exists():
        print(f"ERROR: manifest not found: {manifest_path}", file=sys.stderr)
        return 2

    db_path: Path | None = None
    args = argv[2:]
    while args:
        a = args.pop(0)
        if a == "--db":
            if not args:
                print("ERROR: --db needs a path", file=sys.stderr)
                return 2
            db_path = Path(args.pop(0))
        else:
            print(f"ERROR: unknown arg {a!r}", file=sys.stderr)
            return 2

    with open(manifest_path, "rb") as fh:
        m = json.load(fh)
    if not isinstance(m, dict):
        print("ERROR: manifest is not a JSON object", file=sys.stderr)
        return 2

    # Required fields (the canonical schema).
    missing = [f for f in ("app_version", "db_version", "db_url", "download_url")
               if not m.get(f)]
    if missing:
        print(f"ERROR: manifest is missing required fields: {missing}", file=sys.stderr)
        return 2

    # Cross-check `app_version` against tauri.conf.json — the v0.8.0 retag
    # taught us exactly this foot-gun: the manifest says one version, the
    # binary another. Soft fail (warn) instead of hard fail so manual
    # re-signing of an older manifest is still possible.
    tauri_conf = Path(__file__).resolve().parent.parent / "src-tauri" / "tauri.conf.json"
    if tauri_conf.exists():
        try:
            with open(tauri_conf, "rb") as fh:
                tc = json.load(fh)
            expected = tc.get("version")
            if expected and expected != m["app_version"]:
                print(
                    f"WARNING: manifest app_version={m['app_version']!r} does not match "
                    f"tauri.conf.json version={expected!r}. Sign anyway? "
                    "(Ctrl+C to abort, Enter to continue)",
                    file=sys.stderr,
                )
                try:
                    input()
                except (KeyboardInterrupt, EOFError):
                    return 2
        except (OSError, ValueError) as exc:
            print(f"warning: could not cross-check tauri.conf.json: {exc}", file=sys.stderr)

    # Compute + record the DB checksum if requested. It's signed but the v0.8.0
    # client does NOT yet verify the DB bytes against it (per the agreed
    # "strict-signature only" policy) — a future client can add that check
    # without changing the manifest format.
    if db_path is not None:
        if not db_path.exists():
            print(f"ERROR: --db file not found: {db_path}", file=sys.stderr)
            return 2
        m["db_sha256"] = sha256_file(db_path)
    elif "db_sha256" not in m:
        m["db_sha256"] = ""

    # Default optional fields so the signed payload is well-defined.
    m.setdefault("notes", "")

    # Sign — strip any prior signature first so re-signing is idempotent.
    m.pop("signature", None)
    payload = build_signed_payload(m)
    priv = load_private_key()
    sig = priv.sign(payload)
    m["signature"] = b64encode(sig).decode("ascii")

    # Pretty-print so a human can sanity-check the served file.
    out = json.dumps(m, ensure_ascii=False, indent=2) + "\n"
    tmp = manifest_path.with_suffix(manifest_path.suffix + ".part")
    tmp.write_text(out, encoding="utf-8")
    os.replace(tmp, manifest_path)

    print(f"Signed {manifest_path}")
    print(f"  app_version : {m['app_version']}")
    print(f"  db_version  : {m['db_version']}")
    print(f"  db_sha256   : {m['db_sha256'] or '(empty)'}")
    print(f"  signature   : {m['signature'][:16]}...{m['signature'][-8:]} ({len(m['signature'])} chars)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
