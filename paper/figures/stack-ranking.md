# Stack-Ranking Figure

Source data for `stack-ranking.eps`.

Use in LaTeX:

```tex
\includegraphics[width=\linewidth]{figures/stack-ranking.eps}
```

## Rubric Scores

| Rank | Label | Stack | ANSS |
|---:|---|---|---:|
| 1 | RUST | Rust core + TS/React/Vite + PostgreSQL | 94 |
| 2 | GOSV | Go services + TS/React/Vite + PostgreSQL | 90 |
| 3 | NET | C#/.NET + TS/React/Vite + PostgreSQL | 89 |
| 4 | TSRG | TS product plane + Rust/Go compute cells + PostgreSQL | 88 |
| 5 | JVM | Kotlin/Java JVM + TS/React/Vite + PostgreSQL | 87 |

## Design Notes

- EPS is hand-authored PostScript with no external image dependency.
- Bounding box is `0 44 840 454`.
- The x-axis starts at `85`.
- ANSS means Agent-Native Stack Score, the weighted jankurai 100-point rubric.
- All entries use TypeScript/React/Vite for the product surface and PostgreSQL for durable truth; streaming buses are workload-specific additions rather than baseline stack identity.
- The PDF is generated from the EPS with `epstopdf`.
