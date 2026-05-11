# UX Routes: Periodic Cron

## `/admin/cron`
**State**: List of active schedules.
- Data required: `[CronSchedule]`
- Empty state: "No scheduled jobs configured. Add a cron expression to begin."
- Loaded state: Table displaying ID, Expression, Next Run, and Status.
- Actions: Pause, Resume, Delete, Trigger Now.

## `/admin/cron/new`
**State**: Creation form.
- Data required: None.
- Loaded state: Form with inputs for `id`, `expression`, and `payload_ref`.
- Validation: Surface cron parsing errors in real-time ("Invalid cron expression format").
- Submission: Navigates back to `/admin/cron` on success.

## `/admin/cron/:schedule_id/edit`
**State**: Modification form.
- Data required: `CronSchedule`
- Loaded state: Form allowing `expression` and `is_paused` updates.
- Success: Toast "Schedule updated successfully."
