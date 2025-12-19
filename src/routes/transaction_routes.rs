use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::{get, post};
use crate::AppState;
use crate::controllers::transaction_controllers::{credit_money, debit_money, get_all_transactions, get_transaction_details, transfer_money};

pub async fn transaction_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/credit", post(credit_money))
        .route("/debit", post(debit_money))
        .route("/transfer", post(transfer_money))
        .route("/", get(get_all_transactions))
        .route("/{transaction_id}", get(get_transaction_details))
        .layer(middleware::from_fn(crate::middlewares::authentication_middleware::auth_check))
}