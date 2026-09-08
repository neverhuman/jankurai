#!/usr/bin/env python3
"""Fail publication if products, provenance, checksums, or signatures are absent."""
import hashlib
from pathlib import Path
import sys
import tarfile

dist = Path(sys.argv[1])
version = Path("VERSION").read_text().strip()
assets = ["family.lock", "Cargo.lock", "jankurai-installer.sh", f"jankurai-ux-qa-{version}.tgz"]
for target in ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin"):
    assets.append(f"provenance-{target}.json")
    for product in ("jankurai", "tuiwright"):
        name = f"{product}-{version}-{target}"
        assets.append(f"{name}.tar.gz")
        with tarfile.open(dist / f"{name}.tar.gz") as archive:
            allowed = {name, *(f"{name}/{p}" for p in
                        (product, "LICENSE", "family.lock", "Cargo.lock", "provenance.json"))}
            if set(archive.getnames()) != allowed:
                sys.exit(f"unexpected release payload: {name}")
            if any(m.issym() or m.islnk() for m in archive):
                sys.exit(f"linked release payload: {name}")
for name in assets:
    path = dist / name
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if (dist / f"{name}.sha256").read_text() != f"{digest}  {name}\n":
        sys.exit(f"checksum mismatch: {name}")
    if not (dist / f"{name}.sigstore.bundle").stat().st_size:
        sys.exit(f"missing signature: {name}")
print(f"verified release inventory: {len(assets)} products and metadata files")
