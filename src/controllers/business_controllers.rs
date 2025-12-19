use std::sync::Arc;
use axum::{Extension, Json};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::AppState;
use crate::models::bussiness_models::BusinessState;
use crate::models::common::{AccountId, ApiResponse};

pub async fn get_business_details(State(app_state): State<Arc<AppState>>, Extension(business_account): Extension<AccountId>) -> impl IntoResponse {
    tracing::info!("going to get the business details based on the business_id, {}", business_account.account_id) ;

    match app_state.database_connector.get_business_account_by_id(business_account.account_id).await {
        Ok(business) => {
            tracing::info!("got the business details {:?}", business) ;
            (
                StatusCode::OK,
                Json(ApiResponse::success(business))
            )
        },
        Err(e) => {
            tracing::error!("error was {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<BusinessState>::error(e.to_string())),
            )
        }
    }
}