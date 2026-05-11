# Jankurai Paper Source

Canonical release source is `paper/jankurai.tex` and the TeX files under
`paper/tex/`.

## Source Roles

- `paper/jankurai.tex`: thin build wrapper and section order.
- `paper/tex/frontmatter.tex`: document class, packages, macros, title page,
  abstract, and shared styling.
- `paper/tex/sections/`: main paper sections.
- `paper/tex/appendices/`: appendix material.
- `paper/tex/generated/`: generated TeX tables. Do not edit these by hand.
- `paper/outline.md`: human planning outline.
- `paper/jankurai.md`: agent companion context, not a generator input.

## Generated Tables

- `paper/tex/generated/public_repo_score_tables.tex`
  - Source: `paper/data/public-repo-scores-20260506T014156Z.json`
  - Command: `cargo run -p jankurai -- paper public-repo-scores --source paper/data/public-repo-scores-20260506T014156Z.json --out paper/tex/generated/public_repo_score_tables.tex`
- `paper/tex/generated/vibe_coverage_table.tex`
  - Source: `agent/vibe-coverage.toml`
  - Command: `cargo run -p jankurai -- vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --tex paper/tex/generated/vibe_coverage_table.tex`
- `paper/tex/generated/conformance_results_table.tex`
  - Source: `conformance/fixtures` and `conformance/expected`
  - Command: `cargo run -p jankurai -- conformance run --fixtures conformance/fixtures --expected conformance/expected --out target/jankurai/conformance-results.json --md target/jankurai/conformance-results.md --tex paper/tex/generated/conformance_results_table.tex`

## Build

```bash
just paper
```

The generated PDF is `paper/jankurai.pdf`; it is generator output and should not
be hand-edited.
