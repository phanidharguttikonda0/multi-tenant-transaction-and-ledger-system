use std::sync::Arc;
use axum::Router;
use axum::routing::{delete, get, post, put};
use crate::AppState;
use crate::controllers::webhooks_controllers::{delete_webhook, get_webhooks, register_webhook, update_webhook};

pub async fn webhook_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_webhooks))
        .route("/", post(register_webhook))
        .route("/{webhook_id}", delete(delete_webhook))
        .route("/{webhook_id}", put(update_webhook))
}