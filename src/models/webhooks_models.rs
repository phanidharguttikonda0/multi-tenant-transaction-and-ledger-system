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


use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct WebhookEventRow {
    pub id: i64,
    pub webhook_id: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub status: String,           // pending | delivered | failed
    pub attempt_count: i32,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct WebhookRow {
    pub id: i64,
    pub business_id: i64,
    pub url: String,
    pub secret: String,
    pub status: String,           // active | disabled
}

