use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct Business {
    pub name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BusinessState {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>
}