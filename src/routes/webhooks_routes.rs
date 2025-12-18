use std::sync::Arc;
use axum::Router;
use axum::routing::{delete, get, post, put};
use crate::AppState;

pub async fn webhook_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(|| async {
            tracing::info!("list all registered webhooks") ;
        }))
        .route("/", post(|| async {
            tracing::info!("register an webhook") ;
        }))
        .route("/{webhook_id}", delete(|| async {
            tracing::info!("deletes the specified webhook") ;
        }))
        .route("/{webhook_id}", put(|| async {
            tracing::info!("update webhook configuration") ;
        }))
}