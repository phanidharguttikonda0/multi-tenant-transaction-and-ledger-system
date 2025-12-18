use axum::Router;
use axum::routing::get;
use tracing_appender::non_blocking;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    let (non_blocking, _guard) = non_blocking(std::io::stdout());
    tracing_subscriber::fmt().with_writer(non_blocking).init();
    tracing::info!("Initialized tracing subscriber with async writer") ;
    dotenv().ok();
    tracing::info!("Loaded .env file") ;
    let port = std::env::var("PORT").unwrap_or("4545".to_string()) ;
    tracing::info!("Starting server on port {}", port) ;
    let tcp_listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    
    tracing::info!("Server started") ;
    axum::serve(tcp_listener, top_level_routes().await).await.unwrap() ;
}


async fn top_level_routes() -> Router {
    Router::new()
        .route("/health",get(|| async {
            tracing::info!("Health check") ;
            "OK" 
        }))
}
