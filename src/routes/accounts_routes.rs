use axum::Router;
use axum::routing::{get, post};

pub async fn accounts_routes() -> Router {
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