use axum::Router;
use axum::routing::{delete, get, post};

pub async fn admin_routes() -> Router {
    Router::new()
        .route("/businesses", post(|| async {
            tracing::info!("Creates a new business");
            "OK" }
        ))
        .route("/businesses", get(|| async {
            tracing::info!("Gets all businesses");
            "OK"
        }))
        .route("/businesses/{business_id}/api-keys", post(|| async {
            tracing::info!("generates new api keys for a business") ;
            "OK"
        }))
        .route("/api-keys/{key_id}/rotate", post(|| async {
            tracing::info!("Rotates an existing api key") ;
        }))
        .route("/api-keys/{key_id}", delete(|| async {
            tracing::info!("revokes an existing api key") ;
        }))
        .route("/admin-api-keys", post(|| async {
            tracing::info!("generate admin api keys") ;
        }))
        .route("/admin-api-keys/{key_id}", delete(|| async {
            tracing::info!("revoke admin api keys") ;
        }))
}