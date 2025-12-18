use axum::Router;
use axum::routing::{get, post};

pub async fn transaction_routes() -> Router {
    Router::new()
        .route("/credit", post(|| async {
            tracing::info!("Credit money to an account") ;
        }))
        .route("/debit", post(|| async {
            tracing::info!("Debit money from an account") ;
        }))
        .route("/transfer", post(|| async {
            tracing::info!("Transfer money between accounts") ;
        }))
        .route("/", get(|| async {
            tracing::info!("Get all transactions of a business") ;
        }))
        .route("/{transaction_id}", get(|| async {
            tracing::info!("Get transaction details of the specific transaction") ;
        }))
}