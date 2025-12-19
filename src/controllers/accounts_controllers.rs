
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
    tracing::info!("getting all accounts for business {}", business.account_id) ;
    match app_state.database_connector
        .get_accounts_by_business(business.account_id)
        .await
    {
        Ok(accounts) => (
            StatusCode::OK,
            Json(ApiResponse::success(accounts)),
        ),
        Err(e) => {
            tracing::error!("error occurred while getting accounts {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Vec<Account>>::error(e.to_string())),
            )
        },
    }
}


pub async fn get_account_balance(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {

    tracing::info!("getting balance for account {}", account_id) ;
    if !app_state.database_connector
        .validate_account_ownership(business.account_id, account_id)
        .await
        .unwrap_or(false)
    {
        tracing::warn!("unauthorized account tried to get balance for account {}", account_id) ;
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<String>::error("Unauthorized account".into())),
        );
    }
    tracing::info!("getting balance") ;
    match app_state.database_connector.get_account_balance(account_id).await {
        Ok(balance) => (
            StatusCode::OK,
            Json(ApiResponse::success(balance.to_string())),
        ),
        Err(e) => {
            tracing::error!("got an error while getting balance {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        },
    }
}


pub async fn get_account_details(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Path(account_id): Path<i64>,
) -> impl IntoResponse {
    tracing::info!("getting account details for account {}", account_id) ;
    if !app_state.database_connector
        .validate_account_ownership(business.account_id, account_id)
        .await
        .unwrap_or(false)
    {
        tracing::warn!("unauthorized account tried to get account details for account {}", account_id) ;
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<Account>::error("Unauthorized account".into())),
        );
    }
    tracing::info!("going to get account details") ;
    match app_state.database_connector.get_account_details(account_id).await {
        Ok(account) => (
            StatusCode::OK,
            Json(ApiResponse::success(account)),
        ),
        Err(e) => {
            tracing::error!("got an error while getting account details {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Account>::error(e.to_string())),
            )
        },
    }
}


pub async fn create_account(
    State(app_state): State<Arc<AppState>>,
    Extension(business): Extension<AccountId>,
    Json(new_account): Json<NewAccount>,
) -> impl IntoResponse {
    tracing::info!("creating new account for business {}", business.account_id) ;
    if new_account.name.trim().is_empty() {
        tracing::warn!("account name is empty") ;
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Account name is required".into())),
        );
    }

    if new_account.currency.len() != 3 {
        tracing::warn!("invalid currency") ;
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Invalid currency code".into())),
        );
    }
    tracing::info!("creating a new account") ;
    match app_state.database_connector
        .create_account(business.account_id, new_account)
        .await
    {
        Ok(account_id) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(account_id)),
        ),
        Err(e) => {
            tracing::error!("error while creating a new account {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(e.to_string())),
            )
        },
    }
}
