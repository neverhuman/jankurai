#!/usr/bin/env python3
"""Portable checkout and build commands for the locked Jankurai family."""
from __future__ import annotations

import argparse
import contextlib
import fcntl
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib


class FamilyError(RuntimeError):
    pass


def run(*args, cwd=None, capture=False, check=True, env=None):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env,
                            text=True, capture_output=capture, check=False)
    if check and result.returncode:
        raise FamilyError(f"command failed ({result.returncode}): {' '.join(map(str, args))}"
                          + (f"\n{result.stderr.strip()}" if capture else ""))
    return result.stdout.strip() if capture and check else result


def git(path, *args, **kwargs):
    return run("git", "-C", path, *args, **kwargs)


def read_toml(path):
    return tomllib.loads(path.read_text())


def write_atomic(path, content):
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as out:
            out.write(content)
            out.flush()
            os.fsync(out.fileno())
        os.replace(name, path)
    finally:
        if os.path.exists(name):
            os.unlink(name)


class Family:
    def __init__(self, hub):
        self.hub = Path(hub).resolve()
        self.root = self.hub.parent
        self.manifest = read_toml(self.hub / "repos.manifest.toml")
        self.lock = read_toml(self.hub / "family.lock")
        self.repos = self.manifest["repo"]
        self.pins = {r["repo"]: r for r in self.lock["repo"]}
        self.fusion = self.hub / ".fusion"
        self.validate()

    def validate(self):
        m = self.manifest
        if m["schema_version"] != "2.0.0" or m["authority_forge"] != "github":
            raise FamilyError("expected GitHub family manifest schema 2.0.0")
        names = [r["name"] for r in self.repos]
        if len(names) != len(set(names)) or set(names) != set(m["required_repos"]):
            raise FamilyError("duplicate or missing family repository")
        if len(names) != m["expected_repo_count"]:
            raise FamilyError("family repository count differs from manifest")
        if set(self.pins) != set(names) - {"jankurai"} or len(self.pins) != len(self.lock["repo"]):
            raise FamilyError("family.lock must pin each component exactly once")
        for repo in self.repos:
            name = repo["name"]
            if not re.fullmatch(r"jankurai(?:-[a-z]+)*", name):
                raise FamilyError(f"invalid repository name: {name}")
            url = f"https://github.com/{m['public_owner']}/{name}.git"
            if repo["path"] != name or repo["github"] != url or repo["hosted"] != url:
                raise FamilyError(f"{name}: expected relative canonical path and GitHub URL")
            if repo["required_check"] != f"{name}/required" or repo["default_branch"] != "main":
                raise FamilyError(f"{name}: invalid branch/check contract")
            if "tag" in repo or "commit" in repo:
                raise FamilyError("component revision pins belong only in family.lock")
            if name in self.pins:
                pin = self.pins[name]
                if not re.fullmatch(r"[a-f0-9]{40}", pin["commit"]):
                    raise FamilyError(f"{name}: malformed locked commit")
                if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", pin["tag"]):
                    raise FamilyError(f"{name}: malformed immutable tag")

    def components(self):
        return [r for r in self.repos if r["name"] != "jankurai"]

    def path(self, repo):
        return self.hub if repo["name"] == "jankurai" else self.root / repo["path"]

    def existing(self, repo):
        path = self.path(repo)
        if not path.exists() and not path.is_symlink():
            return False
        if path.is_symlink() or not (path / ".git").is_dir() or (path / ".git").is_symlink():
            raise FamilyError(f"{path}: expected a canonical primary checkout, not a symlink/worktree")
        if git(path, "rev-parse", "--show-toplevel", capture=True) != str(path):
            raise FamilyError(f"{path}: aliased checkout")
        return True

    def clean(self, path):
        if git(path, "status", "--porcelain", capture=True):
            raise FamilyError(f"{path.name}: dirty checkout; obtain a stopped-head handoff")
        gd = Path(git(path, "rev-parse", "--absolute-git-dir", capture=True))
        for marker in ("index.lock", "MERGE_HEAD", "CHERRY_PICK_HEAD", "rebase-merge", "rebase-apply"):
            if (gd / marker).exists():
                raise FamilyError(f"{path.name}: Git operation in progress ({marker})")

    @contextlib.contextmanager
    def operation(self):
        gd = Path(git(self.hub, "rev-parse", "--absolute-git-dir", capture=True))
        with (gd / "family-operation.lock").open("a") as lock:
            try:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError as exc:
                raise FamilyError("another family command owns this checkout") from exc
            yield

    def fetch_pin(self, repo):
        path, pin = self.path(repo), self.pins[repo["name"]]
        # Never force tags: a moved dependency tag must be investigated.
        git(path, "fetch", "--no-tags", repo["github"],
            f"refs/tags/{pin['tag']}:refs/tags/{pin['tag']}")
        resolved = git(path, "rev-parse", f"refs/tags/{pin['tag']}^{{commit}}", capture=True)
        if resolved != pin["commit"]:
            raise FamilyError(f"{repo['name']}: immutable tag no longer matches family.lock")

    def bootstrap(self, restore=False):
        # Preflight the entire existing family before making any checkout changes.
        existing = [r for r in self.components() if self.existing(r)]
        if restore:
            for repo in existing:
                self.clean(self.path(repo))
        for repo in self.components():
            path, pin = self.path(repo), self.pins[repo["name"]]
            if not path.exists():
                # Clone directly into the canonical destination; never make worktrees.
                run("git", "clone", "--no-checkout", "--origin", "origin", repo["github"], path)
                self.fetch_pin(repo)
                git(path, "checkout", "--detach", pin["commit"])
            elif restore:
                self.fetch_pin(repo)
        if restore:
            for repo in existing:
                path, pin = self.path(repo), self.pins[repo["name"]]
                head = git(path, "rev-parse", "HEAD", capture=True)
                if head != pin["commit"] and git(path, "merge-base", "--is-ancestor", head,
                                                  pin["commit"], check=False).returncode:
                    raise FamilyError(f"{repo['name']}: restoring lock would discard ahead/divergent commits")
            for repo in existing:
                path, pin = self.path(repo), self.pins[repo["name"]]
                self.clean(path)
                if git(path, "rev-parse", "HEAD", capture=True) != pin["commit"]:
                    git(path, "checkout", "--detach", pin["commit"])

    def fuse(self, copy_lock=True):
        links = self.fusion / "components"
        links.mkdir(parents=True, exist_ok=True)
        members, patches = [], {}
        for repo in self.components():
            path = self.path(repo)
            if not self.existing(repo):
                raise FamilyError(f"missing component: {repo['name']}; run setup")
            link = links / repo["name"]
            if link.is_symlink():
                if link.resolve() != path:
                    raise FamilyError(f"refusing to replace mismatched link: {link}")
            elif link.exists():
                raise FamilyError(f"refusing to overwrite existing directory: {link}")
            else:
                link.symlink_to(os.path.relpath(path, links), target_is_directory=True)
            manifest = path / "Cargo.toml"
            if manifest.exists():
                for member in read_toml(manifest)["workspace"]["members"]:
                    rel = f"components/{repo['name']}/{member}"
                    package = read_toml(path / member / "Cargo.toml")["package"]["name"]
                    members.append(rel)
                    patches.setdefault(repo["github"], []).append((package, rel))
        cargo = '[workspace]\nresolver = "2"\nmembers = ' + json.dumps(members, indent=2) + '\n'
        for url, entries in patches.items():
            cargo += f'\n[patch.{json.dumps(url)}]\n'
            for package, rel in entries:
                cargo += f'{package} = {{ path = {json.dumps(rel)} }}\n'
        write_atomic(self.fusion / "Cargo.toml", cargo)
        if copy_lock:
            if not (self.hub / "Cargo.lock").is_file():
                raise FamilyError("missing committed aggregate Cargo.lock")
            shutil.copyfile(self.hub / "Cargo.lock", self.fusion / "Cargo.lock")
        write_atomic(self.fusion / "dev.sh", '#!/usr/bin/env bash\nset -euo pipefail\n'
                     'exec bash "$(dirname "${BASH_SOURCE[0]}")/../scripts/family.sh" "${@:-build}"\n')
        (self.fusion / "dev.sh").chmod(0o755)

    def dependencies(self):
        run("cargo", "fetch", "--locked", cwd=self.fusion)
        for repo in self.components():
            path = self.path(repo)
            if (path / "package-lock.json").exists():
                run("npm", "ci", cwd=path)

    def build(self, release=False, target=None):
        self.bootstrap()
        self.fuse()
        args = ["cargo", "build", "--locked", "-p", "jankurai", "-p", "tuiwright-cli"]
        if release:
            args.append("--release")
        if target:
            args.extend(["--target", target])
        run(*args, cwd=self.fusion)
        ux = self.root / "jankurai-tools-ux"
        if not (ux / "node_modules").is_dir():
            run("npm", "ci", cwd=ux)
        run("npm", "run", "build", cwd=ux)

    def check(self):
        self.build()
        self.dependencies()
        run("cargo", "test", "--workspace", "--locked", cwd=self.fusion)
        ux = self.root / "jankurai-tools-ux"
        run("npm", "exec", "--", "playwright", "install", "chromium", cwd=ux)
        run("npm", "test", cwd=ux)
        env = os.environ.copy()
        env["PATH"] = str(self.fusion / "target/debug") + os.pathsep + env["PATH"]
        for repo in self.components():
            run("bash", "scripts/ci-local.sh", "required", cwd=self.path(repo), env=env)
        run("bash", "ops/ci/integration.sh", cwd=self.hub, env=env)

    def status(self):
        for repo in self.repos:
            path = self.path(repo)
            if not self.existing(repo):
                print(f"{repo['name']}: missing")
                continue
            head = git(path, "rev-parse", "HEAD", capture=True)
            branch = git(path, "branch", "--show-current", capture=True) or "detached"
            pin = self.pins.get(repo["name"], {}).get("commit")
            state = "hub" if pin is None else "locked" if head == pin else f"differs from lock {pin[:12]}"
            print(f"{repo['name']}: {branch} {head[:12]} ({state})")
            dirty = git(path, "status", "--short", capture=True)
            if dirty:
                print(dirty)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["setup", "pull", "build", "check", "status", "fuse", "validate"])
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--target")
    args = parser.parse_args()
    family = Family(Path(__file__).resolve().parent.parent)
    if args.command == "status":
        family.status()
        return
    if args.command == "validate":
        print("validate-family: ok")
        return
    with family.operation():
        if args.command == "setup":
            family.bootstrap(restore=True)
            family.fuse()
            family.dependencies()
        elif args.command == "build":
            family.build(args.release, args.target)
        elif args.command == "check":
            family.check()
        elif args.command == "fuse":
            family.bootstrap()
            family.fuse()
        elif args.command == "pull":
            from family_update import update
            update(family)


if __name__ == "__main__":
    try:
        main()
    except (FamilyError, OSError, ValueError, KeyError) as exc:
        sys.exit(f"family: {exc}")
