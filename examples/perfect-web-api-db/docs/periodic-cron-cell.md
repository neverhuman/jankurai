# Periodic Cron Cell

The `periodic-cron` cell provides deterministic evaluation of scheduled intervals. It decouples the evaluation of "when should this run?" from "how do we execute this?", delegating the execution phase to the `background-job` cell.

## Architecture

1. **Schedule Registration**: Schedules are stored in `periodic_cron_schedules` with a standard cron expression.
2. **Deterministic Evaluation**: A single leader (or isolated workers) polls `claim_due_schedules`. If `next_run_at <= NOW()`, the schedule is claimed.
3. **Dispatch**: The scheduler enqueues a new background job using `payload_ref`.
4. **Advancement**: The scheduler calculates the exact next tick based on the cron expression and updates `next_run_at`.

## Usage

Extend the `CronScheduleStore` trait to implement polling and dispatch loops within your adapter layer. By relying on the `PeriodicCronSchedulePolicy`, you ensure that daylight savings time bounds, missed ticks, and execution overlaps are handled centrally without mutating business logic.
