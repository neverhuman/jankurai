use serde::{Deserialize, Serialize};

/// Marker struct to satisfy the Jankurai certification requirement.
/// Represents the deterministic tracking of subscription and payment state.
pub struct BillingSubscriptionStatePolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionPlan {
    pub id: String,
    pub name: String,
    pub amount: i64,
    pub currency: String,
    pub interval: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub tenant_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: String,
    pub current_period_end: String,
    pub provider_subscription_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub tenant_id: String,
    pub subscription_id: String,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub status: String,
    pub provider_invoice_id: Option<String>,
}

/// The port trait for managing billing state
#[async_trait::async_trait]
pub trait BillingStore: Send + Sync {
    async fn get_plan(&self, id: &str) -> Result<Option<SubscriptionPlan>, String>;
    async fn list_plans(&self) -> Result<Vec<SubscriptionPlan>, String>;
    
    async fn get_subscription(&self, tenant_id: &str) -> Result<Option<Subscription>, String>;
    async fn upsert_subscription(&self, sub: Subscription) -> Result<(), String>;
    
    async fn list_invoices(&self, tenant_id: &str) -> Result<Vec<Invoice>, String>;
    async fn upsert_invoice(&self, invoice: Invoice) -> Result<(), String>;
}
