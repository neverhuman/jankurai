#!/usr/bin/env python3
"""Validate portable metadata, locked inputs, and committed dependency boundaries."""
import argparse
from pathlib import Path
import re
import sys

from family import Family, FamilyError, git, read_toml


def validate_tree(path):
    # Inspect tracked inputs, never dependency caches, generated fusion trees,
    # or the deliberately adversarial conformance corpus.
    tracked = git(path, "ls-files", "-z", capture=True).split("\0")
    for relative in tracked:
        file = path / relative
        if not file.is_file() or relative.startswith("conformance/fixtures/"):
            continue
        if file.name == "Cargo.toml":
            data = read_toml(file)
            tables = [data]
            while tables:
                table = tables.pop()
                for value in table.values():
                    if not isinstance(value, dict):
                        continue
                    tables.append(value)
                    if "git" in value and ("branch" in value or not ("tag" in value or "rev" in value)):
                        raise FamilyError(f"{file}: Git dependencies require immutable tags/revisions")
                    if "path" in value:
                        target = (file.parent / value["path"]).resolve()
                        if not target.is_relative_to(path):
                            raise FamilyError(f"{file}: committed cross-repo path dependency")
        if relative.startswith(".github/workflows/") or relative == "action.yml":
            for use in re.findall(r"\buses:\s*[\"']?([^\s\"'#]+)", file.read_text()):
                if not use.startswith("./") and not re.fullmatch(r"[^@]+@[a-f0-9]{40}", use):
                    raise FamilyError(f"{file}: action must be pinned to a full commit SHA: {use}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkouts", action="store_true")
    args = parser.parse_args()
    family = Family(Path(__file__).resolve().parent.parent)
    if not (family.hub / "Cargo.lock").is_file():
        raise FamilyError("aggregate Cargo.lock is missing")
    read_toml(family.hub / "Cargo.lock")
    for repo in family.repos:
        path = family.path(repo)
        if not family.existing(repo):
            if args.checkouts:
                raise FamilyError(f"missing component: {repo['name']}")
            continue
        validate_tree(path)
        for required in ("AGENTS.md", "SPLIT.md", "agent/owner-map.json", "agent/test-map.json",
                         "agent/generated-zones.toml", "scripts/ci-local.sh", "ops/ci/required.sh"):
            if not (path / required).is_file():
                raise FamilyError(f"{repo['name']}: missing {required}")
        if args.checkouts and repo["name"] in family.pins:
            pin = family.pins[repo["name"]]
            if git(path, "rev-parse", "HEAD", capture=True) != pin["commit"]:
                raise FamilyError(f"{repo['name']}: HEAD differs from accepted lock")
            if git(path, "rev-parse", f"refs/tags/{pin['tag']}^{{commit}}", capture=True) != pin["commit"]:
                raise FamilyError(f"{repo['name']}: tag differs from accepted lock")
    print("validate-family: ok")


if __name__ == "__main__":
    try:
        main()
    except (FamilyError, OSError, ValueError) as exc:
        sys.exit(f"validate-family: {exc}")
