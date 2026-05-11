## Agent Tooling Rules

Agent-specific tools matter, but they must not become separate constitutions. Cursor, Codex, Claude Code, GitHub Copilot, Gemini CLI, Jules, Antigravity-style IDEs, Aider-style CLI agents, and open-source agents should all receive the same standard through tool-specific adapters.

### Canonical Source And Adapters

The canonical source is:

```text
AGENTS.md
agent/JANKURAI_STANDARD.md
agent/owner-map.json
agent/test-map.json
agent/generated-zones.toml
agent/audit-policy.toml
docs/agent-native-standard.md
```

Tool adapters should be generated or reviewed against that source:

| Tool family | Adapter | Rule |
| --- | --- | --- |
| Codex | `AGENTS.md`, nested `AGENTS.override.md` when needed | root routes, local files specialize |
| Cursor | `.cursor/rules/*.mdc` | use globs and always-on rules only for hard gates |
| Claude Code | `CLAUDE.md` or `.claude/CLAUDE.md` | team instructions only, no personal state |
| GitHub Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | repo-wide file stays short; path files own detail |
| Gemini CLI | `GEMINI.md` | mirrors canonical standard and memory inspection commands |
| Antigravity-style IDEs | mission/control docs | adapter only until official rule behavior stabilizes |
| Aider-style agents | repo map and token budget | maps route search; standard routes edits |

Hard rule: no adapter may grant permission that the canonical standard denies. If tool files disagree, CI and the audit win.

### Prompt Files Are Not Policy

Prompt files are useful only when they are backed by executable checks. A root instruction that says "do not edit generated files" is weaker than a generated-zone manifest plus CI diff check. A rule saying "keep files small" is weaker than a scanner that reports hard LOC caps. A note saying "run tests" is weaker than a test map that routes changed files to commands.

The right hierarchy is:

1. hard constraints in code, type systems, contracts, and database constraints
2. CI gates and audit rules
3. machine-readable maps
4. local repair docs
5. root agent instructions

If a rule appears only at level five, it is guidance. If it matters, promote it.

### Token Minimization Rules

Token minimization is not about starving the agent. It is about reducing irrelevant context so the agent can spend attention on the owner cell and proof lane.

| Practice | Rule |
| --- | --- |
| root instructions | keep under 100 lines where possible and under 180 hard |
| local instructions | scope by path and keep them shorter than the files they govern |
| docs | split decisions, runbooks, exceptions, and research instead of one giant manual |
| command output | use filtered wrappers for broad commands, with raw-output escape hatch |
| generated files | exclude from default context unless source or output is being audited |
| search | prefer `rg`, owner maps, and symbol maps before broad reading |
| reports | write JSON for agents and Markdown for humans |
| citations | keep source ledgers separate from main narrative |

This is where RTK belongs in jankurai. It is a concrete local wrapper for token-heavy command output. The general rule is broader: filter safely, preserve enough evidence, and allow full raw output when security, auditability, or debugging requires it.

### Versioned Rules

The standard must version quickly because agent tooling is changing quickly. Every adopted repo should pin:

```json
{
  "jankurai_standard": "0.3.0",
  "audit_min_version": "0.3.0",
  "audit_update_channel": "stable",
  "fail_on": ["critical", "high"],
  "advisory_on": ["medium", "low"]
}
```

The audit should warn when the pinned standard is stale. It should not surprise-break CI on a minor update. Breaking rule changes need release notes, migration guidance, and sample repairs.
