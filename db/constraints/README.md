# db/constraints

Status: active routing surface
Owner: standard
Last reviewed: 2026-05-02
Applies to: `db/constraints/`

Put named constraints, check constraints, foreign key notes, and row level security policy here.

Document the invariant each constraint protects and the application or adapter boundary that enforces it.
Call out which adapter or application-owned transaction is responsible for setting or repairing the data, and note any rollback, backfill, or lock implications for changing the constraint later.
