extern crate core;

mod routes;
mod middlewares;
mod models;
mod services;
mod controllers;

use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use std::time::Duration;
use tracing_appender::non_blocking;
use dotenv::dotenv;
use tokio::sync::mpsc::unbounded_channel;
use crate::controllers::admin_controllers::create_bootstraped_admin;
use crate::controllers::business_controllers::get_business_details;
use crate::models::event_queue::WebhookQueueMessage;
use crate::routes::accounts_routes::accounts_routes;
use crate::routes::admin_routes::admin_routes;
use crate::routes::transaction_routes::transaction_routes;
use crate::routes::webhooks_routes::webhook_routes;
use crate::services::db_operations::DbOperations;
use crate::services::webhook_events_executor::{redis_expiry_subscriber, webhook_worker};

pub struct AppState {
   pub database_connector: DbOperations,
   pub event_queue: tokio::sync::mpsc::UnboundedSender<WebhookQueueMessage>,
    pub redis_client: redis::Client,
}

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
    let database_connector = DbOperations::new().await ;
    let (event_tx, event_rx) = unbounded_channel::<WebhookQueueMessage>();
    let redis_client = redis::Client::open("redis://127.0.0.1:6379/").unwrap();
    let state = Arc::new(AppState {
        database_connector,
        event_queue: event_tx,
        redis_client,
    }) ;
    tracing::info!("spawning webhook worker") ;
    tokio::spawn(webhook_worker(
        state.clone(),
        event_rx,
    ));

    tracing::info!("spawning redis expiry subscriber events listener") ;
    let state_ = state.clone() ;
    tokio::spawn(async move {
        loop {
            if let Err(e) = redis_expiry_subscriber(state_.clone()).await {
                tracing::error!("Redis expiry listener failed: {:?}", e);
            }
            tracing::warn!("Restarting listener in 2 seconds...");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    Router::new()
        .route("/health",get(|| async {
            tracing::info!("Health check") ;
            "OK"
        }))
        .route("/get-business-account", get(get_business_details))
        .route("/_internal/bootstrap/admin", get(create_bootstraped_admin))
        .nest("/admin", admin_routes().await)
        .nest("/accounts", accounts_routes().await)
        .nest("/transaction", transaction_routes().await)
        .nest("/webhooks", webhook_routes().await)
        .with_state(state)
}
