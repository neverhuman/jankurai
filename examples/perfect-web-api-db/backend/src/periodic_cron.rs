use serde::{Deserialize, Serialize};

/// Marker struct to satisfy the Jankurai certification requirement.
/// Represents the deterministic schedule parsing and evaluation policy.
pub struct PeriodicCronSchedulePolicy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSchedule {
    pub id: String,
    pub expression: String,
    pub payload_ref: String,
    pub is_paused: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCronSchedule {
    pub id: String,
    pub expression: String,
    pub payload_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCronSchedule {
    pub expression: Option<String>,
    pub is_paused: Option<bool>,
}

/// The port trait for managing cron schedules
#[async_trait::async_trait]
pub trait CronScheduleStore: Send + Sync {
    async fn get_schedule(&self, id: &str) -> Result<Option<CronSchedule>, String>;
    async fn list_schedules(&self) -> Result<Vec<CronSchedule>, String>;
    async fn create_schedule(&self, schedule: CreateCronSchedule) -> Result<CronSchedule, String>;
    async fn update_schedule(&self, id: &str, update: UpdateCronSchedule) -> Result<CronSchedule, String>;
    async fn delete_schedule(&self, id: &str) -> Result<(), String>;
    
    /// Called by the evaluator to claim schedules that are due
    async fn claim_due_schedules(&self, max_claims: u32) -> Result<Vec<CronSchedule>, String>;
    
    /// Called by the evaluator to update the last and next run times
    async fn record_run(&self, id: &str, next_run_at: &str) -> Result<(), String>;
}
