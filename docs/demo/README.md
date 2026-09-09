# Audit demo GIFs

`scripts/demo/render-audit-gif.mjs` is the only producer. CI runs it on every
fast lane. It writes:

| File | Role |
| --- | --- |
| `audit-readme.gif` | README embed (smaller, bright palette) |
| `audit-1080p.gif` | 1920×1080 lossless-palette GIF, under 50MB |
| `audit-demo.json` | exact sizes, score source, and paths |

Colors are a 16-entry saturated table with solid cells. There is no dithering
or dim gray wash. If `jankurai` is on `PATH` (or `JANKURAI_BIN`), the score
card uses a live advisory audit of a disposable fixture; otherwise the
renderer uses the committed hub badge score.

```sh
node scripts/demo/render-audit-gif.mjs
```
