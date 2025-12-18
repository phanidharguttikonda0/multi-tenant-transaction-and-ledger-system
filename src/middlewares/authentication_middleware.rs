use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

pub async fn auth_check(mut req: Request, next: Next) -> Response {
    tracing::info!("Auth check") ;
    next.run(req).await
}

pub async fn admin_auth_check(mut req: Request, next: Next) -> Response {
    tracing::info!("Admin auth check") ;
    next.run(req).await
}