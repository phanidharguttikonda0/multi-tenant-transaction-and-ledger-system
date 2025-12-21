use std::sync::Arc;
use axum::{middleware, Router};
use axum::routing::{get, post};
use crate::AppState;
use crate::controllers::accounts_controllers::{create_account, get_account_balance, get_account_details, get_accounts};

pub async fn accounts_routes(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_accounts))
        .route("/{account_id}", get(get_account_details))
        .route("/{account_id}/balance", get(get_account_balance))
        .route("/", post(create_account))
        .layer(middleware::from_fn_with_state(app_state, crate::middlewares::authentication_middleware::auth_check))
}