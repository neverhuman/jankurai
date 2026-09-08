#!/usr/bin/env python3
"""Trusted, hub-only PR publication. Never executes candidate component code."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tomllib

from family import Family, FamilyError
from family_update import successful

REPO = "neverhuman/jankurai"
FILES = {"family.lock", "Cargo.lock"}


def api(endpoint, payload=None, method=None):
    args = ["gh", "api", endpoint]
    if method:
        args += ["--method", method]
    if payload is not None:
        args += ["--input", "-"]
    result = subprocess.run(args, input=json.dumps(payload) if payload is not None else None,
                            text=True, capture_output=True, check=False)
    if result.returncode:
        raise FamilyError(result.stderr.strip())
    return json.loads(result.stdout) if result.stdout.strip() else None


def content(path, ref):
    data = api(f"repos/{REPO}/contents/{path}?ref={ref}")
    return base64.b64decode(data["content"]).decode()


def validate_candidate(family, candidate, base):
    old, new = tomllib.loads(base["family.lock"]), tomllib.loads(candidate["family.lock"])
    if set(old) != set(new) or any(old[k] != new[k] for k in old if k != "repo"):
        raise FamilyError("updater may only change component tag/commit pins")
    if len(new["repo"]) != len(old["repo"]):
        raise FamilyError("candidate changed family membership")
    changed = False
    for previous, pin in zip(old["repo"], new["repo"]):
        if set(previous) != set(pin) or any(previous[k] != pin[k] for k in previous if k not in ("tag", "commit")):
            raise FamilyError("candidate changed repository metadata/order")
        if pin == previous:
            continue
        changed = True
        sha = pin["commit"]
        if not re.fullmatch(r"[0-9a-f]{40}", sha) or pin["tag"] != f"ci-{sha}":
            raise FamilyError("candidate needs an immutable exact-SHA CI tag")
        repo = next(r for r in family.repos if r["name"] == pin["repo"])
        ref = api(f"repos/{repo['slug']}/git/ref/tags/{pin['tag']}")
        if ref["object"]["type"] != "commit" or ref["object"]["sha"] != sha:
            raise FamilyError("candidate CI tag mismatch")
        comparison = api(f"repos/{repo['slug']}/compare/{sha}...{repo['default_branch']}")
        if comparison["status"] not in ("ahead", "identical") or not successful(repo, sha):
            raise FamilyError("candidate is not a successful default-branch revision")
    if not changed:
        raise FamilyError("candidate contains no component update")
    cargo = tomllib.loads(candidate["Cargo.lock"])
    if cargo.get("version") != 4 or not isinstance(cargo.get("package"), list):
        raise FamilyError("invalid aggregate Cargo lock")


def branch_for(candidate):
    digest = hashlib.sha256((candidate["family.lock"] + "\0" + candidate["Cargo.lock"]).encode()).hexdigest()
    return "automation/family-" + digest[:24]


def publish(family, directory):
    candidate = {}
    for name in FILES:
        path = directory / name
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 2_000_000:
            raise FamilyError("invalid candidate artifact")
        candidate[name] = path.read_text()
    base_sha = api(f"repos/{REPO}/git/ref/heads/main")["object"]["sha"]
    base = {name: content(name, base_sha) for name in FILES}
    validate_candidate(family, candidate, base)
    branch = branch_for(candidate)
    existing = api(f"repos/{REPO}/pulls?state=open&head=neverhuman:{branch}")
    if existing:
        print(existing[0]["html_url"])
        return
    tree = []
    for name in sorted(FILES):
        blob = api(f"repos/{REPO}/git/blobs", {"content": candidate[name], "encoding": "utf-8"})
        tree.append({"path": name, "mode": "100644", "type": "blob", "sha": blob["sha"]})
    base_tree = api(f"repos/{REPO}/git/commits/{base_sha}")["tree"]["sha"]
    tree_sha = api(f"repos/{REPO}/git/trees", {"base_tree": base_tree, "tree": tree})["sha"]
    commit = api(f"repos/{REPO}/git/commits", {
        "message": "Update validated Jankurai family locks", "tree": tree_sha, "parents": [base_sha]})
    api(f"repos/{REPO}/git/refs", {"ref": f"refs/heads/{branch}", "sha": commit["sha"]})
    pr = api(f"repos/{REPO}/pulls", {
        "title": "Update validated Jankurai component revisions", "head": branch, "base": "main",
        "body": "Automated family update. Each changed component has a successful required check and "
                "an immutable CI tag. The candidate locks passed combined integration in a disposable "
                "CI checkout. Ordinary PR CI must pass before a protected merge."})
    print(pr["html_url"])


def merge(family):
    actor = api("user")["login"]
    hub = next(r for r in family.repos if r["name"] == "jankurai")
    for pr in api(f"repos/{REPO}/pulls?state=open&base=main&per_page=100"):
        if (pr["user"]["login"] != actor or pr["head"]["repo"]["full_name"] != REPO
                or not pr["head"]["ref"].startswith("automation/family-") or pr["draft"]):
            continue
        files = api(f"repos/{REPO}/pulls/{pr['number']}/files?per_page=100")
        if not files or any(f["filename"] not in FILES or f["status"] != "modified" for f in files):
            continue
        candidate = {name: content(name, pr["head"]["sha"]) for name in FILES}
        base = {name: content(name, pr["base"]["sha"]) for name in FILES}
        if branch_for(candidate) != pr["head"]["ref"]:
            continue
        validate_candidate(family, candidate, base)
        if not successful(hub, pr["head"]["sha"]):
            continue
        # GitHub enforces strict protected-branch checks; sha prevents a head race.
        result = api(f"repos/{REPO}/pulls/{pr['number']}/merge", {
            "sha": pr["head"]["sha"], "merge_method": "squash"}, method="PUT")
        if not result.get("merged"):
            raise FamilyError("protected updater merge was refused")
        print(f"Merged {pr['html_url']} at {result['sha']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["publish", "merge"])
    parser.add_argument("--directory", type=Path, default=Path("target/candidate"))
    args = parser.parse_args()
    try:
        family = Family(Path(__file__).resolve().parent.parent)
        if args.command == "publish":
            publish(family, args.directory)
        else:
            merge(family)
    except (FamilyError, KeyError, ValueError, OSError) as exc:
        sys.exit(f"family automation: {exc}")
