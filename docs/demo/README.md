# Audit demo GIFs

These GIFs replay observed phases from a real audit of an authored sample
repository. A sample failure stays FAIL with its measured score. This is a
terminal-style rendering of recorded events, not a screen recording and not an
audit of this hub checkout.

Normal CI writes fresh output under `target/audit-demo/` and uploads it. It does
not rewrite the tracked files in this directory. Copy a verified render here
only when refreshing the README preview.

```sh
node scripts/demo/generate-demo.mjs /absolute/path/to/jankurai "$PWD/target/audit-demo"
```
