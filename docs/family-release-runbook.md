# Family release and provisioning

GitHub is the manifest-selected authority. Use `scripts/provision-family.sh --dry-run`
to inspect repository creation and protection, and `--apply` to provision a clean,
claimed family. Existing repositories and refs are preserved; the existing hub
is migrated through a PR based on its GitHub `main`.

Do not create Git worktrees. Busy or dirty canonical checkouts require a stopped-head
handoff. Unmerged branches remain preserved refs; import never force-pushes a
branch or tag. `scripts/project-family-manifest.sh --apply` creates the optional
workspace-root projection and preserves a changed predecessor by SHA-256 in
`.bundles/manifest-projection-custody/`.

See [automation and rotation](github-automation.md) and [release validation](release.md).
