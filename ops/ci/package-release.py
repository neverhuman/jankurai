#!/usr/bin/env python3
"""Package public products and the exact locked family that produced them."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile

hub = Path(__file__).resolve().parents[2]
target = sys.argv[1]
if target not in ("x86_64-unknown-linux-gnu", "aarch64-apple-darwin"):
    sys.exit("unsupported release target")
version = (hub / "VERSION").read_text().strip()
dist = hub / "dist"
dist.mkdir(exist_ok=True)


def output(*args):
    return subprocess.check_output(args, text=True, cwd=hub).strip()


provenance = {
    "schema": "jankurai.release/v1", "version": version, "target": target,
    "repository": "https://github.com/neverhuman/jankurai",
    "commit": output("git", "rev-parse", "HEAD"),
    "family_lock_sha256": hashlib.sha256((hub / "family.lock").read_bytes()).hexdigest(),
    "cargo_lock_sha256": hashlib.sha256((hub / "Cargo.lock").read_bytes()).hexdigest(),
    "rustc": output("rustc", "--version"), "cargo": output("cargo", "--version"),
    "node": output("node", "--version"),
    "workflow_run": os.environ.get("GITHUB_RUN_ID"),
}
provenance_file = dist / f"provenance-{target}.json"
provenance_file.write_text(json.dumps(provenance, indent=2) + "\n")
for binary in ("jankurai", "tuiwright"):
    source = hub / ".fusion/target" / target / "release" / binary
    subprocess.run([source, "--version"], check=True)
    with tempfile.TemporaryDirectory(prefix="release-stage-", dir=dist) as temporary:
        stage = Path(temporary) / f"{binary}-{version}-{target}"
        stage.mkdir()
        shutil.copy2(source, stage / binary)
        for metadata in ("family.lock", "Cargo.lock", "LICENSE"):
            shutil.copyfile(hub / metadata, stage / metadata)
        shutil.copyfile(provenance_file, stage / "provenance.json")
        with tarfile.open(dist / f"{stage.name}.tar.gz", "w:gz") as archive:
            archive.add(stage, arcname=stage.name)
if target == "x86_64-unknown-linux-gnu":
    subprocess.run(["npm", "pack", "--workspace", "@jankurai/ux-qa", "--pack-destination", str(dist)],
                   cwd=hub.parent / "jankurai-tools-ux", check=True)
    for metadata in ("family.lock", "Cargo.lock", "jankurai-installer.sh"):
        shutil.copyfile(hub / metadata, dist / metadata)
for asset in sorted(dist.iterdir()):
    if asset.is_file() and not asset.name.endswith((".sha256", ".sigstore.bundle")):
        (dist / f"{asset.name}.sha256").write_text(
            hashlib.sha256(asset.read_bytes()).hexdigest() + "  " + asset.name + "\n")
