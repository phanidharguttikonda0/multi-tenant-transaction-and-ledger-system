use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info, warn};
use crate::AppState;
use crate::models::event_queue::WebhookQueueMessage;

pub async fn webhook_worker(
    app_state: Arc<AppState>,
    mut rx: UnboundedReceiver<WebhookQueueMessage>,
) {
    info!("Webhook worker started");

    while let Some(msg) = rx.recv().await {
        let event_id = msg.webhook_event_id;

        info!("processing webhook_event_id={}", event_id);

        // Loading  webhook event
        let event = match app_state
            .database_connector
            .get_webhook_event(event_id)
            .await
        {
            Ok(Some(e)) => e,
            Ok(None) => {
                error!("webhook event {} not found", event_id);
                continue;
            }
            Err(e) => {
                error!("db error loading webhook event {}: {}", event_id, e);
                continue;
            }
        };

        // Skipping the terminal states
        if event.status == "delivered" || event.status == "failed" {
            tracing::warn!("which are already sent and which are already exhausted failed, so we don't need to send them again") ;
            tracing::info!("ignoring those tasks") ;
            continue;
        }
        tracing::info!("getting webhook config") ;
        // Loading webhook config
        let webhook = match app_state
            .database_connector
            .get_webhook(event.webhook_id)
            .await
        {
            Ok(Some(w)) => w,
            _ => {
                error!("webhook config missing for event {}", event_id);
                continue;
            }
        };

        // Sending HTTP webhook
        let send_result = send_webhook_http(&webhook, &event).await;

        match send_result {
            Ok(_) => {
                // ✅ Success
                let _ = app_state
                    .database_connector
                    .mark_webhook_event_delivered(event_id)
                    .await;

                info!("webhook_event {} delivered", event_id);
            }

            Err(err) => {
                warn!(
                    "webhook_event {} failed attempt {}: {}",
                    event_id, event.attempt_count, err
                );

                // Retrying the logic
                let next = compute_next_retry(event.attempt_count);

                if let Some(next_retry_at) = next {
                    tracing::warn!("a retry failed still there are some retries") ;

                    tracing::info!("updating db with next retry timestamp") ;
                    // Updating DB
                    let _ = app_state
                        .database_connector
                        .schedule_webhook_retry(
                            event_id,
                            next_retry_at,
                        )
                        .await;

                    tracing::info!("adding the key to redis with next TTL") ;
                    // Scheduling Redis TTL(Total Time to Live)
                    let _ = schedule_redis_retry(
                        &app_state.redis_client,
                        event_id,
                        next_retry_at,
                    )
                        .await;
                } else {
                    // Exhausted all retry logic
                    tracing::warn!("exhausted all retry logic of the webhook") ;
                    let _ = app_state
                        .database_connector
                        .mark_webhook_event_failed(event_id)
                        .await;
                }
            }
        }
    }
}


use reqwest::Client;

async fn send_webhook_http(
    webhook: &WebhookRow,
    event: &WebhookEventRow,
) -> Result<(), String> {
    let client = Client::new();

    let res = client
        .post(&webhook.url)
        .json(&event.payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        Ok(())
    } else {
        tracing::warn!("http {} {}", res.status(), res.text().await.unwrap_or_default());
        tracing::warn!("webhook failed with status, so we need to send an notification to the user, via mail or anything else") ;
        Err(format!("http {}", res.status()))
    }
}

use chrono::{DateTime, Utc, Duration};

fn compute_next_retry(attempts: i32) -> Option<DateTime<Utc>> {
    match attempts {
        0 => Some(Utc::now() + Duration::seconds(30)),
        1 => Some(Utc::now() + Duration::minutes(2)),
        2 => Some(Utc::now() + Duration::minutes(10)),
        3 => Some(Utc::now() + Duration::hours(1)),
        _ => None, // exhausted
    }
}


use redis::AsyncCommands;
use crate::models::webhooks_models::{WebhookEventRow, WebhookRow};

async fn schedule_redis_retry(
    redis_client: &redis::Client,
    event_id: i64,
    retry_at: chrono::DateTime<chrono::Utc>,
) -> redis::RedisResult<()> {
    let mut conn = redis_client.get_async_connection().await?;

    let ttl = (retry_at - chrono::Utc::now()).num_seconds().max(1);

    let key = format!("webhook:retry:{}", event_id);

    conn.set_ex(key, "", ttl as usize).await?;

    Ok(())
}


pub async fn enqueue_pending_webhooks(app_state: Arc<AppState>) {
    let events = app_state
        .database_connector
        .get_pending_webhook_events()
        .await
        .unwrap_or_default();

    for event_id in events {
        let _ = app_state.event_queue.send(WebhookQueueMessage {
            webhook_event_id: event_id,
        });
    }
}


pub async fn redis_expiry_subscriber(app_state: Arc<AppState>) {
    info!("Starting Redis expiry subscriber");

    let client = app_state.redis_client.clone();
    let mut pubsub = match client.get_async_pubsub().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to connect to Redis: {}", e);
            return;
        }
    };

    // Listen to key expiry events
    if let Err(e) = pubsub.subscribe("__keyevent@0__:expired").await
    {
        error!("Failed to subscribe to Redis expiry events: {}", e);
        return;
    }

    info!("Subscribed to Redis key expiry events");

    while let Ok(msg) = pubsub.on_message().await {
        let expired_key: String = match msg.get_payload() {
            Ok(v) => v,
            Err(_) => continue,
        };

        tracing::info!("got an expired event key: {}", expired_key);
        // We only care about webhook retry keys, remaining keys not needed here
        if let Some(event_id) = parse_webhook_retry_key(&expired_key) {
            info!(
                "Redis TTL expired for webhook_event_id={}",
                event_id
            );

            // Push event_id back into an unbounded channel
            tracing::info!("adding event_id back to the unbounded channel") ;
            let _ = app_state.event_queue.send(
                WebhookQueueMessage {
                    webhook_event_id: event_id,
                }
            );
        }
    }
}

fn parse_webhook_retry_key(key: &str) -> Option<i64> {
    // Expected format: webhook:retry:{event_id}
    tracing::info!("extracting the event_id from the key: {}", key) ;
    let prefix = "webhook:retry:";

    if key.starts_with(prefix) {
        key[prefix.len()..].parse::<i64>().ok()
    } else {
        None
    }
}

