use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Credit,
    Debit,
    Transfer,
}

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub id: i64,
    pub business_id: i64,
    pub from_account_id: Option<i64>,
    pub to_account_id: Option<i64>,
    pub txn_type: String,
    pub amount: Decimal,
    pub status: String,
    pub reference_id: Option<String>,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
}


#[derive(Debug, Deserialize)]
pub struct CreditRequest {
    pub to_account_id: i64,
    pub amount: Decimal,
    pub reference_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub struct DebitRequest {
    pub from_account_id: i64,
    pub amount: Decimal,
    pub reference_id: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
pub struct TransferRequest {
    pub from_account_id: i64,
    pub to_account_id: i64,
    pub amount: Decimal,
    pub reference_id: Option<String>,
    pub idempotency_key: String,
}
