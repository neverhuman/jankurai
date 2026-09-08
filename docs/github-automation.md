# GitHub family automation

The portable `repos.manifest.toml` declares the 15 repository identities, relative
checkout paths, GitHub URLs, default branches, and aggregate check names.
Component revision pins live only in `family.lock`; the assembled dependency
resolution lives in the committed aggregate `Cargo.lock`.

Each component publishes a lightweight `ci-<full commit SHA>` tag after its
`<repo>/required` GitHub Actions job succeeds on `main`. Tag rulesets prevent
updates and deletion. The hub also verifies that an eligible tag points to its
named commit, that the commit remains reachable from `main`, and that the exact
commit has a successful aggregate check produced by GitHub Actions.

The hourly `family-update` workflow tests candidate revisions in an automatically
removed standalone CI checkout. It executes the complete family checks and keeps
the accepted locks unchanged on failure. The resulting PR contains only
`family.lock` and `Cargo.lock`. A separate hourly merge job considers only the
expected automation actor, the content-derived `automation/family-*` branch name,
the same hub repository, the permitted changed files, eligible component pins,
and a successful `jankurai/required` check for the exact PR head. GitHub enforces
strict protected-branch checks and the merge API additionally binds the head SHA.
A later hourly run merges a candidate after normal PR CI has completed; the
PAT-authenticated merge triggers post-merge CI.

## Secret custody and rotation

`FAMILY_AUTOMATION_TOKEN` is a repository secret on **neverhuman/jankurai only**.
Only the trusted publication and merge jobs receive it. Build/test jobs receive
no supplied automation token; candidate subprocesses also strip token variables.
Publication parses bounded lock artifacts and writes Git blobs through the API;
it never checks out or executes candidate component code with that token.

The supplied token expires **October 8, 2026**. Rotate it before that date by
updating the hub repository secret and `FAMILY_AUTOMATION_TOKEN_EXPIRES` variable.
The hourly rotation job fails with an annotation starting 14 days before expiry,
so the required action is visible in Actions. After rotation, dispatch the
updater and verify PR CI, protected merge, and post-merge CI.

A token-authenticated PR allows normal CI to run automatically, as described in
[GitHub's workflow trigger documentation](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).
