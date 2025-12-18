use std::sync::Arc;
use axum::extract::State;
use axum::{Form, Json};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use crate::{models, AppState};
use crate::models::common::ApiResponse;
use crate::services::other_services::{generate_api_key};

pub async fn create_business(
    State(app_state): State<Arc<AppState>>,
    Json(new_business): Json<models::bussiness_models::Business>,
) -> impl IntoResponse {

    match app_state.database_connector.create_new_business(&new_business.name).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(id)),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<i64>::error(err.to_string())),
        ),
    }
}


pub async fn get_businesses(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {

    match app_state.database_connector.get_businesses().await {
        Ok(businesses) => (
            StatusCode::OK,
            Json(ApiResponse::success(businesses)),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<models::bussiness_models::BusinessState>>::error(
                err.to_string(),
            )),
        ),
    }
}



pub async fn generate_api_keys(
    State(app_state): State<Arc<AppState>>,
    Json(business_id): Json<i64>,
) -> impl IntoResponse {

    if !app_state.database_connector
        .validate_business_id(business_id)
        .await
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<String>::error("Invalid business".to_string())),
        );
    }

    let (raw_key, hashed_key) = generate_api_key();

    match app_state.database_connector.store_api_key(business_id, &hashed_key).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(raw_key)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}


pub async fn rotate_api_key(
    State(app_state): State<Arc<AppState>>,
    Json((key_id, business_id)): Json<(i64, i64)>,
) -> impl IntoResponse {

    let (raw_key, hashed_key) = generate_api_key();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    };

    let res = async {
        app_state.database_connector
            .expire_api_key_txn(&mut tx, key_id, expires_at)
            .await?;

        app_state.database_connector
            .store_api_key_txn(&mut tx, business_id, &hashed_key)
            .await?;

        Ok::<_, sqlx::Error>(())
    }.await;

    match res {
        Ok(_) => {
            tx.commit().await.unwrap();
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(raw_key)),
            )
        }
        Err(e) => {
            tx.rollback().await.unwrap();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}




pub async fn revoke_api_key(
    State(app_state): State<Arc<AppState>>,
    Json(key_id): Json<i64>,
) -> impl IntoResponse {

    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    };

    match app_state.database_connector.revoke_api_key_txn(&mut tx, key_id).await {
        Ok(_) => {
            tx.commit().await.unwrap();
            (
                StatusCode::OK,
                Json(ApiResponse::success("revoked".to_string())),
            )
        }
        Err(e) => {
            tx.rollback().await.unwrap();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}



pub async fn generate_admin_api_keys(
    State(app_state): State<Arc<AppState>>,
    Json(admin_id): Json<i64>,
) -> impl IntoResponse {

    let (raw_key, hashed_key) = generate_api_key();

    match app_state.database_connector.store_admin_api_key(admin_id, &hashed_key).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(raw_key)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}



pub async fn revoke_admin_api_key(
    State(app_state): State<Arc<AppState>>,
    Json(key_id): Json<i64>,
) -> impl IntoResponse {

    match app_state.database_connector.revoke_admin_api_key(key_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success("revoked".to_string())),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e.to_string())),
        ),
    }
}



pub async fn create_bootstraped_admin(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {

    match app_state.database_connector.create_admin_account("default_admin").await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(id)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<i64>::error(e.to_string())),
        ),
    }
}

