# Audit demo GIFs

These are first-class public recordings of a real `jankurai audit --full --mode standard`
against an authored sample repository. The measured PASS/FAIL and score are
never substituted. The README embeds the preview; this directory holds the
full-resolution sibling.

| File | Role | Geometry |
| --- | --- | --- |
| [audit-readme.gif](audit-readme.gif) | README preview | 960×540 |
| [audit-1080p.gif](audit-1080p.gif) | Full-resolution recording | 1920×1080 |
| [audit-demo.json](audit-demo.json) | Receipt: palette, hashes, timing, outcome | — |

Public paths:

- Preview: `docs/demo/audit-readme.gif`
- Full resolution: `docs/demo/audit-1080p.gif`

## Guarantees

- Exact 16-color palette from `scripts/demo/gif-encode.mjs`. No dither, no
  extra quantization, no dimming.
- GIF89a lossless LZW of those palette indexes.
- Every file, including the 1080p recording, stays under 50 MB.
- Timeline is the observed process. A sample FAIL stays FAIL.

## Rebuild

CI records a fresh sample into `target/audit-demo/` and verifies it. That job
does not rewrite these tracked files. Refresh the public catalog only from a
verified render:

```sh
node scripts/demo/generate-demo.mjs /absolute/path/to/jankurai "$PWD/target/audit-demo"
node scripts/demo/publish-demo.mjs "$PWD/target/audit-demo/rendered" "$PWD/docs/demo"
npm run demo:verify
```
