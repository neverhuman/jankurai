# IDE Integrations

Agent and IDE adapters should be thin pointers to `agent/JANKURAI_STANDARD.md` and `agent/MASTER_PLAN.md`.

Planning instructions also stay canonical: adapters must point to `agent/MASTER_PLAN.md#detailed-planner-protocol` instead of duplicating long planning prompts. That keeps Cursor, Copilot, Claude, Gemini, and generic agent adapters aligned while allowing the project to improve planning policy in one file.

Supported generated pointers:

- `.cursor/rules/jankurai.mdc`
- `.github/instructions/jankurai.instructions.md`
- `.claude/skills/jankurai/SKILL.md`
- Gemini, Antigravity, and Aider-style adapters through the same canonical pointer

Agent verification is first-class now:

```bash
jankurai agent verify
jankurai adapters verify
jankurai context-pack --task "..." --out target/jankurai/context-pack.json
jankurai repair-plan --from agent/repo-score.json --out target/jankurai/repair-plan.json
```

Durable policy belongs in `docs/` or `agent/`, not in per-tool prompt forks.
