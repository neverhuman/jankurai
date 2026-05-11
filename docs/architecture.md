# jankurai Architecture

jankurai is a paper, standard, and audit workspace. The product standard it
defines is:

```text
Rust core + TypeScript/React/Vite product surface + PostgreSQL truth
+ generated contracts + exception-only Python AI/data service
```

New implementation should be Rust-first. Agents must not write Python for repo
tools, proof lanes, product services, general backend glue, authorization, or
production database writes. Python is allowed only for rare advanced ML/data
library work that has no practical Rust/TypeScript/service alternative, stays
boxed to `python/ai-service`, and carries a dated exception.

The canonical architecture is documented in:

- `docs/agent-native-standard.md`
- `agent/JANKURAI_STANDARD.md`
- `paper/tex/sections/09_winner_architecture.tex`

The Markdown files under `paper/sections/` are legacy-only planning companions.
They are not canonical release sources.

Local workspace ownership:

| Path | Role |
| --- | --- |
| `paper/` | canonical TeX manuscript, figures, citation ledgers, legacy companions |
| `tools/` | dependency-free audit script |
| `docs/` | mission, standard, research, release, audit doctrine |
| `agent/` | machine-readable maps and agent bootstrap |
| `reference/` | read-only copied source corpus |
| `tips/` | short reusable guidance distilled from the paper |

Agents should prefer `agent/owner-map.json` and `agent/test-map.json` for
changes, then route to the smallest proof lane.
