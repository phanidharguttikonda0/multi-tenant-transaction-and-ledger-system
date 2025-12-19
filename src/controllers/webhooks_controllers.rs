use axum::extract::{Path, State};
use std::sync::Arc;
use axum::{Extension, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::AppState;
use crate::models::bussiness_models::Business;
use crate::models::common::{AccountId, ApiResponse};
use crate::models::webhooks_models::{CreateWebhookRequest, UpdateWebhookRequest};

pub async fn get_webhooks(
    State(app_state): State<Arc<AppState>>,
    Extension(business_account): Extension<AccountId>,
) -> impl IntoResponse {
    tracing::info!(
        "getting webhooks for business {}",
        business_account
    );

    match app_state
        .database_connector
        .get_webhooks_by_business(business_account.into())
        .await
    {
        Ok(webhooks) => (
            StatusCode::OK,
            Json(ApiResponse::success(webhooks)),
        ),
        Err(e) => {
            tracing::error!("error fetching webhooks {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}



pub async fn register_webhook(
    State(app_state): State<Arc<AppState>>,
    Extension(business_account): Extension<AccountId>,
    Json(req): Json<CreateWebhookRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "registering webhook for business {}",
        business_account
    );

    match app_state
        .database_connector
        .create_webhook(business_account.into(), &req.url)
        .await
    {
        Ok(id) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(id)),
        ),
        Err(e) => {
            tracing::error!("error creating webhook {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}



pub async fn delete_webhook(
    State(app_state): State<Arc<AppState>>,
    Extension(business_account): Extension<AccountId>,
    Path(webhook_id): Path<i64>,
) -> impl IntoResponse {
    tracing::info!(
        "disabling webhook {} for business {}",
        webhook_id,
        business_account
    );

    match app_state
        .database_connector
        .disable_webhook(business_account.into(), webhook_id)
        .await
    {
        Ok(0) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error("webhook not found".into())),
        ),
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success("webhook disabled".to_string())),
        ),
        Err(e) => {
            tracing::error!("error disabling webhook {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}


pub async fn update_webhook(
    State(app_state): State<Arc<AppState>>,
    Extension(business_account): Extension<AccountId>,
    Path(webhook_id): Path<i64>,
    Json(req): Json<UpdateWebhookRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "updating webhook {} for business {}",
        webhook_id,
        business_account
    );

    match app_state
        .database_connector
        .update_webhook(
            business_account.into(),
            webhook_id,
            req.url,
            req.status,
        )
        .await
    {
        Ok(0) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<String>::error("webhook not found".into())),
        ),
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success("webhook updated".to_string())),
        ),
        Err(e) => {
            tracing::error!("error updating webhook {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}
