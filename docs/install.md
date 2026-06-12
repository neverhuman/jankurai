# Install Jankurai

Status: split hub install guide
Owner: Jankurai maintainers
Last reviewed: 2026-06-12
Applies to: `neverhuman/jankurai`

## Release Installer

```bash
curl -fsSL https://github.com/neverhuman/jankurai/releases/download/v1.7.0-split.0/jankurai-installer.sh \
  | JANKURAI_RELEASE_TAG=v1.7.0-split.0 bash
```

The installer verifies the GitHub release, artifact attestation, sha256 file,
and Sigstore bundle before installing the `jankurai` binary.

## Source Development

The hub does not carry core Rust source. For source development, create a fused
workspace:

```bash
git clone https://github.com/neverhuman/jankurai
cd jankurai
./scripts/fuse.sh --source github --all
.fusion/dev.sh build
```

Internal development may use Jeryu instead:

```bash
./scripts/fuse.sh --source jeryu --all
```
