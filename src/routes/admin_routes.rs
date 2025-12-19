use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::{delete, get, post};
use crate::AppState;
use crate::controllers::admin_controllers::{create_business, generate_admin_api_keys, generate_api_keys, get_businesses, revoke_admin_api_key, revoke_api_key, rotate_api_key};

pub async fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/businesses", post(create_business
        ))
        .route("/businesses", get(get_businesses))
        .route("/businesses/{business_id}/api-keys", post(generate_api_keys))
        .route("/api-keys/{key_id}/rotate", post(rotate_api_key))
        .route("/api-keys/{key_id}", delete(revoke_api_key))
        .route("/admin-api-keys", post(generate_admin_api_keys))
        .route("/admin-api-keys/{key_id}", delete(revoke_admin_api_key))
        .layer(middleware::from_fn(crate::middlewares::authentication_middleware::admin_auth_check))
}