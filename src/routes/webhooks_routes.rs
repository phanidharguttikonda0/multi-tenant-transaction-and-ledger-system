use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::{delete, get, post, put};
use crate::AppState;
use crate::controllers::webhooks_controllers::{delete_webhook, get_webhooks, register_webhook, update_webhook};

pub async fn webhook_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_webhooks))
        .route("/", post(register_webhook))
        .route("/{webhook_id}", delete(delete_webhook))
        .route("/{webhook_id}", put(update_webhook))
        .layer(middleware::from_fn_with_state(state.clone(), crate::middlewares::authentication_middleware::auth_check))
}