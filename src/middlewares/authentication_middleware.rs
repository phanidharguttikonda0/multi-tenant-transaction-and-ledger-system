use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use std::sync::Arc;
use crate::AppState;


pub async fn auth_check(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Response {

    // let state = req
    //     .extensions()
    //     .get::<Arc<AppState>>()
    //     .cloned()
    //     .expect("AppState missing");

    let headers = req.headers();
    tracing::info!("received headers") ;
    let raw_key = match extract_bearer_token(headers) {
        Some(k) => k,
        None => {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };
    tracing::info!("Extracted bearer token") ;

    let key_hash = hash_api_key(&raw_key);
    tracing::info!("the key hash is {}", key_hash);
    match state
        .database_connector
        .verify_business_api_key(&key_hash)
        .await
    {
        Ok(business_id) => {
            req.extensions_mut().insert(AccountId { account_id: business_id });
            next.run(req).await
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}


pub async fn admin_auth_check(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    let headers = req.headers();

    let raw_key = match extract_bearer_token(headers) {
        Some(k) => k,
        None => {
            return Err(StatusCode::UNAUTHORIZED.into_response());
        }
    };

    let key_hash = hash_api_key(&raw_key);

    match state
        .database_connector
        .verify_admin_api_key(&key_hash)
        .await
    {
        Ok(admin_id) => {
            req.extensions_mut().insert(AccountId {
                account_id: admin_id,
            });

            Ok(next.run(req).await)
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED.into_response()),
    }
}
use axum::http::{HeaderMap, StatusCode};
use crate::models::common::AccountId;
use crate::services::other_services::hash_api_key;

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

