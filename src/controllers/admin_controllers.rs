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
    tracing::info!("creating new business with name {}", new_business.name) ;
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
    tracing::info!("getting all businesses") ;
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
    tracing::info!("generating api keys for business {}", business_id) ;
    
    if !app_state.database_connector
        .validate_business_id(business_id)
        .await
        .unwrap_or(false)
    {
        tracing::warn!("invalid business id {} passed", business_id) ;
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<String>::error("Invalid business".to_string())),
        );
    }

    let (raw_key, hashed_key) = generate_api_key();
    tracing::info!("got the raw_key and the hashed_key {}, {}", raw_key, hashed_key) ;
    match app_state.database_connector.store_api_key(business_id, &hashed_key).await {
        Ok(_) => {
            tracing::info!("created the new api key for the business sucessfully") ;
            
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(raw_key)),
            )
        },
        Err(e) => {
            tracing::error!("error occurred while creating api key {}", e) ; 
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        },
    }
}


pub async fn rotate_api_key(
    State(app_state): State<Arc<AppState>>,
    Json((key_id, business_id)): Json<(i64, i64)>,
) -> impl IntoResponse {
    tracing::info!("rotating api key for business {}", business_id) ;
    let (raw_key, hashed_key) = generate_api_key();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    tracing::warn!("expires at will be 7 days from the current date and time") ;
    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("got an error while creating a transaction to rotate the api key {}",e) ;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    };
    tracing::info!("created a transaction to rotate the api key for business {}", business_id) ;
    let res = async {
        app_state.database_connector
            .expire_api_key_txn(&mut tx, key_id, expires_at)
            .await?;

        app_state.database_connector
            .store_api_key_txn(&mut tx, business_id, &hashed_key)
            .await?;

        Ok::<_, sqlx::Error>(())
    }.await;
    
    
    tracing::info!("executed expire api key and storing new api key") ;

    match res {
        Ok(_) => {
            tracing::info!("comitting the transaction as both db calls were sucessfull") ;
            tx.commit().await.unwrap();
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(raw_key)),
            )
        }
        Err(e) => {
            tracing::error!("rolling back due to an error {}", e) ;
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
    tracing::info!("revoking api key with id {}", key_id) ;
    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("got an error while creating a transaction to revoke the api key {}",e) ;
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
            tracing::error!("rolling back due to an error {}", e) ;
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
    tracing::info!("generating admin api keys for admin {}", admin_id) ;
    let (raw_key, hashed_key) = generate_api_key();

    match app_state.database_connector.store_admin_api_key(admin_id, &hashed_key).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(raw_key)),
        ),
        Err(e) => {
            tracing::error!("error occurred while creating admin api key {}", e) ; 
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        }
    }
}



pub async fn revoke_admin_api_key(
    State(app_state): State<Arc<AppState>>,
    Json(key_id): Json<i64>,
) -> impl IntoResponse {
    tracing::info!("revoking admin api key with id {}", key_id) ;
    match app_state.database_connector.revoke_admin_api_key(key_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success("revoked".to_string())),
        ),
        Err(e) => {
            tracing::error!("error occurred while revoking admin api key {}", e) ;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(e.to_string())),
            )
        },
    }
}



pub async fn create_bootstraped_admin(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    
    tracing::info!("creating bootstraped admin account") ;

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

