
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::AppState;
use crate::models::accounts_models::{Account, NewAccount};
use crate::models::common::{AccountId, ApiResponse};

pub async fn get_accounts(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
) -> impl IntoResponse {

    match app_state.database_connector
        .get_accounts_by_business(business.account_id)
        .await
    {
        Ok(accounts) => (
            StatusCode::OK,
            Json(ApiResponse::success(accounts)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<Account>>::error(e.to_string())),
        ),
    }
}


pub async fn get_account_balance(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {

    if !app_state.database_connector
        .validate_account_ownership(business.account_id, account_id)
        .await
        .unwrap_or(false)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<String>::error("Unauthorized account".into())),
        );
    }

    match app_state.database_connector.get_account_balance(account_id).await {
        Ok(balance) => (
            StatusCode::OK,
            Json(ApiResponse::success(balance)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}


pub async fn get_account_details(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {

    if !app_state.database_connector
        .validate_account_ownership(business.account_id, account_id)
        .await
        .unwrap_or(false)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<Account>::error("Unauthorized account".into())),
        );
    }

    match app_state.database_connector.get_account_details(account_id).await {
        Ok(account) => (
            StatusCode::OK,
            Json(ApiResponse::success(account)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Account>::error(e.to_string())),
        ),
    }
}


pub async fn create_account(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Json(new_account): Json<NewAccount>,
) -> impl IntoResponse {

    if new_account.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Account name is required".into())),
        );
    }

    if new_account.currency.len() != 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Invalid currency code".into())),
        );
    }

    match app_state.database_connector
        .create_account(business.account_id, new_account)
        .await
    {
        Ok(account_id) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(account_id)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<i64>::error(e.to_string())),
        ),
    }
}
