use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
}

#[derive(Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub status: Option<String>, // active | disabled
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub id: i64,
    pub url: String,
    pub business_id: i64,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
