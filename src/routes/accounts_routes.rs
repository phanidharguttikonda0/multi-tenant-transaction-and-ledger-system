use std::sync::Arc;
use axum::Router;
use axum::routing::{get, post};
use crate::AppState;

pub async fn accounts_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(|| async {
            tracing::info!("Get all accounts of a business") ;
        }))
        .route("/{account_id}", get(|| async {
            tracing::info!("Get account details of the specific account") ;
        }))
        .route("/{account_id}/balance", get(|| async {
            tracing::info!("Get balance of the specific account") ;
        }))
        .route("/", post(|| async {
            tracing::info!("Creates a new account") ;
        }))
}