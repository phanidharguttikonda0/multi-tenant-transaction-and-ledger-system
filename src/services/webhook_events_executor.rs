use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info};
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
            continue;
        }

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
                error!(
                    "webhook_event {} failed attempt {}: {}",
                    event_id, event.attempt_count, err
                );

                // Retrying the logic
                let next = compute_next_retry(event.attempt_count);

                if let Some(next_retry_at) = next {
                    // Updating DB
                    let _ = app_state
                        .database_connector
                        .schedule_webhook_retry(
                            event_id,
                            next_retry_at,
                        )
                        .await;

                    // Scheduling Redis TTL
                    let _ = schedule_redis_retry(
                        &app_state.redis_client,
                        event_id,
                        next_retry_at,
                    )
                        .await;
                } else {
                    // Exhausted all 3 retries
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
