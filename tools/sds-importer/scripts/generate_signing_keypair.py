#!/usr/bin/env python3
"""One-shot generator for the manifest-signing ed25519 keypair.

Run ONCE on your machine. After that, keep the private key safe and use
``sign_manifest.py`` at every deploy.

What it does
------------
- Generates a fresh ed25519 keypair.
- Saves the private seed (32 raw bytes) to
  ``%USERPROFILE%/.maison_drabiec/manifest_signing.ed25519`` with a permission
  hint (Windows file ACLs are coarser than POSIX; the parent dir is created
  with restrictive intent — manual review still recommended).
- Prints the public key as a 64-char hex string ready to paste as a Rust
  ``const`` in ``src-tauri/src/update.rs``.

Safety
------
- REFUSES to overwrite an existing private key. To rotate, move/back-up the old
  file out of the way first — losing it means no future client can verify your
  deploys until you ship a new app version with the new public key embedded.
- The private key is NEVER printed, logged, or copied. It only ever lives at
  the saved path.
"""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def key_dir() -> Path:
    home = os.environ.get("USERPROFILE") or os.path.expanduser("~")
    return Path(home) / ".maison_drabiec"


def key_path() -> Path:
    return key_dir() / "manifest_signing.ed25519"


def main() -> int:
    kp = key_path()
    if kp.exists():
        print(f"REFUSING: {kp} already exists.", file=sys.stderr)
        print("To rotate, move the old file aside first (and remember: until you ", file=sys.stderr)
        print("ship a new app binary with the NEW public key embedded, clients on ", file=sys.stderr)
        print("the OLD pubkey cannot verify manifests you sign with the new key).", file=sys.stderr)
        return 2

    kp.parent.mkdir(parents=True, exist_ok=True)
    # Windows: try to lock the directory to the owner. ACL is coarser than POSIX
    # but this still removes inherited group/everyone rights on a default user
    # profile. Manual review is still recommended.
    try:
        os.chmod(kp.parent, stat.S_IRWXU)
    except Exception:
        pass

    priv = Ed25519PrivateKey.generate()
    priv_seed = priv.private_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PrivateFormat.Raw,
        encryption_algorithm=serialization.NoEncryption(),
    )
    assert len(priv_seed) == 32, "ed25519 raw seed must be 32 bytes"

    pub_bytes = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    assert len(pub_bytes) == 32, "ed25519 raw public key must be 32 bytes"

    # Write the private seed with restrictive intent. Use a temp + rename so a
    # crash mid-write never leaves a half-written file.
    tmp = kp.with_suffix(kp.suffix + ".part")
    with open(tmp, "wb") as fh:
        fh.write(priv_seed)
    try:
        os.chmod(tmp, stat.S_IRUSR | stat.S_IWUSR)
    except Exception:
        pass
    os.replace(tmp, kp)

    print()
    print(f"Private key saved to:  {kp}")
    print(f"Keep a backup (USB / password manager). NEVER commit. NEVER scp to a server.")
    print()
    print("Public key (paste this 64-char hex as SIGNING_PUBLIC_KEY in update.rs):")
    print()
    print(f"    {pub_bytes.hex()}")
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
