use axum::{
    extract::{Request, State, ConnectInfo},
    http::StatusCode,
    middleware::Next,
    response::{Response, IntoResponse},
};
use std::{net::SocketAddr, sync::Arc};
use redis::AsyncCommands;
use crate::AppState;

pub async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let ip = addr.ip().to_string();
    let key = format!("rate_limit:{}", ip);
    
    // Get redis connection
    let mut redis_conn = match state.redis_client.get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Failed to get redis connection: {}", e);
            // Fail open
            return next.run(req).await;
        }
    };

    // Increment counter
    let count: isize = match redis_conn.incr(&key, 1).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Redis INCR error: {}", e);
            return next.run(req).await;
        }
    };

    // If new key (count == 1), set expiry
    if count == 1 {
        // Set expiry to 60 seconds
        let result: redis::RedisResult<()> = redis_conn.expire(&key, 60).await;
        if let Err(e) = result {
             tracing::error!("Redis EXPIRE error: {}", e);
        }
    }

    // Check limit
    if count > 20 {
        tracing::warn!("Rate limit exceeded for IP: {}", ip);
        return (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded").into_response();
    }

    next.run(req).await
}
