#!/usr/bin/env python3
"""Checkout-safety regressions using disposable standalone Git fixtures."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from family import Family, FamilyError, git
from family_update import lock_text


class CheckoutSafety(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="family-ci-test-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.hub = self.root / "jankurai"
        self.repo = self.root / "jankurai-core"
        for path in [self.hub, self.repo]:
            path.mkdir()
            git(path, "init", "-q", "-b", "main")
            git(path, "config", "user.email", "fixture@example.invalid")
            git(path, "config", "user.name", "CI fixture")
            (path / "README.md").write_text("base\n")
            git(path, "add", ".")
            git(path, "commit", "-qm", "base")
        self.sha = git(self.repo, "rev-parse", "HEAD", capture=True)
        git(self.repo, "tag", "fixture-v1")
        rows = []
        for name in ["jankurai", "jankurai-core"]:
            rows.append(f'''[[repo]]
name = "{name}"
path = "{name}"
slug = "neverhuman/{name}"
default_branch = "main"
required_check = "{name}/required"
github = "https://github.com/neverhuman/{name}.git"
hosted = "https://github.com/neverhuman/{name}.git"
''')
        (self.hub / "repos.manifest.toml").write_text('''schema_version = "2.0.0"
authority_forge = "github"
public_owner = "neverhuman"
expected_repo_count = 2
required_repos = ["jankurai", "jankurai-core"]
''' + "\n".join(rows))
        (self.hub / "family.lock").write_text(f'''[[repo]]
repo = "jankurai-core"
tag = "fixture-v1"
commit = "{self.sha}"
''')
        self.family = Family(self.hub)

    def test_build_bootstrap_preserves_dirty_head_and_files(self):
        (self.repo / "README.md").write_text("valuable draft\n")
        with patch.object(self.family, "fetch_pin") as fetch:
            self.family.bootstrap()
        fetch.assert_not_called()
        self.assertEqual((self.repo / "README.md").read_text(), "valuable draft\n")
        self.assertEqual(git(self.repo, "rev-parse", "HEAD", capture=True), self.sha)

    def test_setup_rejects_dirty_before_network_or_ref_changes(self):
        (self.repo / "new-file").write_text("untracked work")
        with patch.object(self.family, "fetch_pin") as fetch:
            with self.assertRaisesRegex(FamilyError, "dirty checkout"):
                self.family.bootstrap(restore=True)
        fetch.assert_not_called()
        self.assertEqual(git(self.repo, "rev-parse", "HEAD", capture=True), self.sha)

    def test_setup_preserves_ahead_commits_and_branch(self):
        (self.repo / "README.md").write_text("ahead\n")
        git(self.repo, "commit", "-qam", "valuable unpublished commit")
        ahead = git(self.repo, "rev-parse", "HEAD", capture=True)
        with patch.object(self.family, "fetch_pin"):
            with self.assertRaisesRegex(FamilyError, "ahead/divergent"):
                self.family.bootstrap(restore=True)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD", capture=True), ahead)
        self.assertEqual(git(self.repo, "branch", "--show-current", capture=True), "main")

    def test_repeated_setup_is_idempotent(self):
        with patch.object(self.family, "fetch_pin"):
            self.family.bootstrap(restore=True)
            self.family.bootstrap(restore=True)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD", capture=True), self.sha)
        self.assertEqual(git(self.repo, "branch", "--show-current", capture=True), "main")

    def test_moved_tag_is_rejected(self):
        self.family.pins["jankurai-core"]["commit"] = "0" * 40
        with patch("family.git", side_effect=lambda p, *a, **kw:
                   "" if a[0] == "fetch" else git(p, *a, **kw)):
            with self.assertRaisesRegex(FamilyError, "immutable tag"):
                self.family.fetch_pin(self.family.components()[0])

    def test_missing_commit_preserves_head(self):
        with patch.object(self.family, "fetch_pin", side_effect=FamilyError("unavailable commit")):
            with self.assertRaisesRegex(FamilyError, "unavailable commit"):
                self.family.bootstrap(restore=True)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD", capture=True), self.sha)

    def test_symlink_checkout_is_rejected(self):
        self.repo.rename(self.root / "unrelated")
        self.repo.symlink_to(self.root / "unrelated", target_is_directory=True)
        with self.assertRaisesRegex(FamilyError, "canonical primary"):
            self.family.bootstrap()

    def test_manifest_rejects_traversal_duplicate_and_revision_pins(self):
        original = copy.deepcopy(self.family.manifest)
        for key, value in [("path", "../jankurai-core"), ("tag", "mutable")]:
            self.family.repos[1][key] = value
            with self.assertRaises(FamilyError):
                self.family.validate()
            self.family.manifest = copy.deepcopy(original)
            self.family.repos = self.family.manifest["repo"]
        self.family.repos.append(self.family.repos[1])
        with self.assertRaises(FamilyError):
            self.family.validate()

    def test_lock_update_changes_only_requested_pin(self):
        original = (self.hub / "family.lock").read_text()
        pins = copy.deepcopy(self.family.pins)
        self.assertEqual(lock_text(original, pins), original)
        pins["jankurai-core"].update(commit="1" * 40, tag="ci-" + "1" * 40)
        result = lock_text(original, pins)
        self.assertIn('commit = "' + "1" * 40 + '"', result)
        self.assertEqual((self.hub / "family.lock").read_text(), original)


if __name__ == "__main__":
    unittest.main()
