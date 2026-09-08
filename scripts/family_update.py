"""Test candidate family locks in disposable standalone CI checkouts."""
import copy
import json
import os
from pathlib import Path
import re
import tempfile

from family import Family, FamilyError, git, run, write_atomic


def api(endpoint):
    return json.loads(run("gh", "api", endpoint, capture=True))


def successful(repo, sha):
    checks = api(f"repos/{repo['slug']}/commits/{sha}/check-runs?filter=latest&per_page=100")
    return any(c["name"] == repo["required_check"] and c["head_sha"] == sha
               and c["status"] == "completed" and c["conclusion"] == "success"
               and c["app"]["slug"] == "github-actions" for c in checks["check_runs"])


def eligible(family, repo):
    path = family.path(repo)
    branch = repo["default_branch"]
    git(path, "fetch", "--no-tags", repo["github"],
        f"refs/heads/{branch}:refs/remotes/origin/{branch}")
    git(path, "fetch", "--no-tags", repo["github"], "refs/tags/ci-*:refs/tags/ci-*")
    # An immutable CI tag is eligible only while reachable from the default branch
    # and backed by the aggregate GitHub Actions check for that exact commit.
    tags = set(git(path, "tag", "--list", "ci-*", capture=True).splitlines())
    for sha in git(path, "rev-list", f"refs/remotes/origin/{branch}", capture=True).splitlines():
        if f"ci-{sha}" in tags:
            tagged = git(path, "rev-parse", f"refs/tags/ci-{sha}^{{commit}}", capture=True)
            if tagged == sha and successful(repo, sha):
                return sha
    return family.pins[repo["name"]]["commit"]


def lock_text(original, pins):
    def replace_block(match):
        block = match.group(0)
        name = re.search(r'^repo = "([^"]+)"$', block, re.M).group(1)
        pin = pins[name]
        block = re.sub(r'^tag = "[^"]+"$', f'tag = "{pin["tag"]}"', block, flags=re.M)
        return re.sub(r'^commit = "[^"]+"$', f'commit = "{pin["commit"]}"', block, flags=re.M)
    return re.sub(r'\[\[repo\]\][\s\S]*?(?=\[\[repo\]\]|\Z)', replace_block, original)


def update(family):
    family.clean(family.hub)
    for repo in family.components():
        if family.existing(repo):
            family.clean(family.path(repo))
    family.bootstrap()
    pins = copy.deepcopy(family.pins)
    before = {p: (family.hub / p).read_text() for p in ("family.lock", "Cargo.lock")}
    for repo in family.components():
        sha = eligible(family, repo)
        path = family.path(repo)
        head = git(path, "rev-parse", "HEAD", capture=True)
        if git(path, "merge-base", "--is-ancestor", head, sha, check=False).returncode:
            raise FamilyError(f"{repo['name']}: candidate would leave divergent local work behind")
        if sha != pins[repo["name"]]["commit"]:
            pins[repo["name"]].update(commit=sha, tag=f"ci-{sha}")
    candidate = lock_text(before["family.lock"], pins)
    if candidate == before["family.lock"]:
        print("family pull: no newer successful component revisions")
        return
    target = family.hub / "target"
    target.mkdir(exist_ok=True)
    # Exact-SHA integration uses an automatically removed standalone checkout.
    # It never registers a Git worktree or moves a developer's component heads.
    with tempfile.TemporaryDirectory(prefix="family-ci-", dir=target) as temporary:
        hub = Path(temporary) / "jankurai"
        run("git", "clone", "--no-hardlinks", "--no-checkout", family.hub, hub)
        git(hub, "checkout", "--detach", git(family.hub, "rev-parse", "HEAD", capture=True))
        write_atomic(hub / "family.lock", candidate)
        sandbox = Family(hub)
        sandbox.bootstrap(restore=True)
        sandbox.fuse(copy_lock=False)
        # Strip publication credentials before running component build/test code.
        env = os.environ.copy()
        for key in ("GH_TOKEN", "GITHUB_TOKEN", "FAMILY_AUTOMATION_TOKEN"):
            env.pop(key, None)
        run("cargo", "generate-lockfile", cwd=sandbox.fusion, env=env)
        cargo = (sandbox.fusion / "Cargo.lock").read_text()
        write_atomic(hub / "Cargo.lock", cargo)
        run("bash", "scripts/family.sh", "check", cwd=hub, env=env)
        for name, content in before.items():
            if (family.hub / name).read_text() != content:
                raise FamilyError(f"{name} changed while testing candidate; accepted locks preserved")
        for repo in family.components():
            family.clean(family.path(repo))
        # No write to accepted locks occurs until all candidate checks pass.
        write_atomic(family.hub / "Cargo.lock", cargo)
        write_atomic(family.hub / "family.lock", candidate)
    print("family pull: validated candidate locks ready for a protected pull request")
