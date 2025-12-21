use std::sync::Arc;
use axum::{Extension, Json};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use sqlx::Row;
use crate::AppState;
use crate::models::common::{AccountId, ApiResponse};
use crate::models::event_queue::WebhookQueueMessage;
use crate::models::transaction_models::{CreditRequest, DebitRequest, Transaction, TransactionStatus, TransactionType, TransferRequest};
use crate::services::db_operations::DbOperations;

pub async fn credit_money(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Json(req): Json<CreditRequest>,
) -> impl IntoResponse {

    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(e.to_string())),
            );
        }
    };

    if let Some(txn_id) =
        DbOperations::check_idempotency(&mut tx, account.account_id, &req.idempotency_key)
            .await
            .unwrap()
    {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::OK,
            Json(ApiResponse::success(txn_id)),
        );
    }

    let (balance, status) =
        match DbOperations::lock_account(&mut tx, req.to_account_id).await {
            Ok(v) => v,
            Err(e) => {
                tx.rollback().await.ok();
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<i64>::error(e.to_string())),
                );
            }
        };

    if status != "active" {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Account is frozen".into())),
        );
    }

    let txn_id = DbOperations::insert_transaction(
        &mut tx,
        account.account_id,
        None,
        Some(req.to_account_id),
        TransactionType::Credit,
        req.amount,
        req.reference_id.clone(),
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    match DbOperations::update_balance(&mut tx, req.to_account_id, balance + req.amount)
        .await {
        Ok(_) => {
            match DbOperations::mark_transaction_status(&mut tx, txn_id, TransactionStatus::Succeeded)
                .await {
                Ok(_) => {

                    // complete the logic of adding an event to the webhook_events
                    //  Build webhook payload
                    let payload = serde_json::json!({
                        "event": "transaction.succeeded",
                        "data": {
                            "transaction_id": txn_id,
                            "type": "credit",
                            "amount": req.amount,
                            "to_account_id": req.to_account_id,
                            "business_id": account.account_id,
                            "reference_id": req.reference_id
                        }
                    });
                    tracing::info!("business_id was {}", account.account_id) ;
                    let webhook_id = match app_state.database_connector.get_webhook(account.account_id).await  {
                        Ok(webhook_id) => webhook_id,
                        Err(err) => {
                            tracing::error!("no webhook was registered to this business account") ;
                            tracing::error!("error was {}", err) ;
                            tx.rollback().await.ok();
                            return (
                                axum::http::StatusCode::NOT_FOUND,
                                Json(ApiResponse::<i64>::error("Register a Webhook First".to_string())),
                            );
                        }
                    };
                    //  Created webhook_event in DB (AFTER transaction logic, BEFORE commit)
                    let webhook_event_id = match app_state
                        .database_connector
                        .create_webhook_event(
                            /* webhook_id */ webhook_id,
                            "transaction.succeeded",
                            payload,
                        )
                        .await
                    {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!("failed to create webhook event {}", e);
                            tx.rollback().await.ok();
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<i64>::error(e.to_string())),
                            );
                        }
                    };






                    tx.commit().await.unwrap();
                    // WebhookQueueMessage, we are going to add to the message to the unbounded
                    app_state.event_queue.send(
                        WebhookQueueMessage {
                            webhook_event_id,
                            webhook_id
                        }
                    ).expect("Unable to add Event Id to the Queue");
                    (
                        axum::http::StatusCode::CREATED,
                        Json(ApiResponse::success(txn_id)),
                    )
                },
                Err(err) => {
                    tracing::error!("Failed to mark transaction status: {:?}", err);
                    tx.rollback().await.ok();
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<i64>::error(err.to_string()))
                    ) ;
                }
            }
        },
        Err(err) => {
            tx.rollback().await.ok();
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(err.to_string()))
                ) ;
        }
    }


}



pub async fn debit_money(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Json(req): Json<DebitRequest>,
) -> impl IntoResponse {

    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(e.to_string())),
            );
        }
    };

    if let Some(txn_id) =
        DbOperations::check_idempotency(&mut tx, account.account_id, &req.idempotency_key)
            .await
            .unwrap()
    {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::OK,
            Json(ApiResponse::success(txn_id)),
        );
    }

    let (balance, status) =
        match DbOperations::lock_account(&mut tx, req.from_account_id).await {
            Ok(v) => v,
            Err(e) => {
                tx.rollback().await.ok();
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<i64>::error(e.to_string())),
                );
            }
        };

    if status != "active" || balance < req.amount {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error(
                "Insufficient balance or frozen account".into(),
            )),
        );
    }

    let txn_id = DbOperations::insert_transaction(
        &mut tx,
        account.account_id,
        Some(req.from_account_id),
        None,
        TransactionType::Debit,
        req.amount,
        req.reference_id.clone(),
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    match DbOperations::update_balance(&mut tx, req.from_account_id, balance - req.amount)
        .await
    {
        Ok(_) => {
            match DbOperations::mark_transaction_status(
                &mut tx,
                txn_id,
                TransactionStatus::Succeeded,
            )
                .await
            {
                Ok(_) => {

                    // -------- WEBHOOK EVENT LOGIC (same as credit) --------

                    // Build webhook payload
                    let payload = serde_json::json!({
                        "event": "transaction.succeeded",
                        "data": {
                            "transaction_id": txn_id,
                            "type": "debit",
                            "amount": req.amount,
                            "from_account_id": req.from_account_id,
                            "business_id": account.account_id,
                            "reference_id": req.reference_id
                        }
                    });
                    let webhook_id = match app_state.database_connector.get_webhook(account.account_id).await  {
                        Ok(webhook_id) => webhook_id,
                        Err(err) => {
                            tracing::error!("no webhook was registered to this business account") ;
                            tracing::error!("error was {}", err) ;
                            tx.rollback().await.ok();
                            return (
                                axum::http::StatusCode::NOT_FOUND,
                                Json(ApiResponse::<i64>::error("Register a Webhook First".to_string())),
                            );
                        }
                    };

                    // Create webhook_event in DB (AFTER transaction logic, BEFORE commit)
                    let webhook_event_id = match app_state
                        .database_connector
                        .create_webhook_event(
                            /* webhook_id */ webhook_id, // same assumption as credit
                            "transaction.succeeded",
                            payload,
                        )
                        .await
                    {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!("failed to create webhook event {}", e);
                            tx.rollback().await.ok();
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<i64>::error(e.to_string())),
                            );
                        }
                    };

                    // ------------------------------------------------------

                    tx.commit().await.unwrap();

                    // Push webhook_event_id to unbounded channel
                    app_state
                        .event_queue
                        .send(WebhookQueueMessage { webhook_event_id, webhook_id })
                        .expect("Unable to add Event Id to the Queue");

                    (
                        axum::http::StatusCode::CREATED,
                        Json(ApiResponse::success(txn_id)),
                    )
                }
                Err(err) => {
                    tracing::error!("Failed to mark transaction status: {:?}", err);
                    tx.rollback().await.ok();
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<i64>::error(err.to_string())),
                    );
                }
            }
        }
        Err(err) => {
            tx.rollback().await.ok();
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(err.to_string())),
            );
        }
    }
}




pub async fn transfer_money(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Json(req): Json<TransferRequest>,
) -> impl IntoResponse {

    let mut tx = match app_state.database_connector.connector.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(e.to_string())),
            );
        }
    };

    if let Some(txn_id) =
        DbOperations::check_idempotency(&mut tx, account.account_id, &req.idempotency_key)
            .await
            .unwrap()
    {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::OK,
            Json(ApiResponse::success(txn_id)),
        );
    }

    let (from_balance, from_status) =
        match DbOperations::lock_account(&mut tx, req.from_account_id).await {
            Ok(v) => v,
            Err(e) => {
                tx.rollback().await.ok();
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<i64>::error(e.to_string())),
                );
            }
        };

    let (to_balance, to_status) =
        match DbOperations::lock_account(&mut tx, req.to_account_id).await {
            Ok(v) => v,
            Err(e) => {
                tx.rollback().await.ok();
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<i64>::error(e.to_string())),
                );
            }
        };

    if from_status != "active" || to_status != "active" || from_balance < req.amount {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error(
                "Insufficient balance or frozen account".into(),
            )),
        );
    }

    let txn_id = DbOperations::insert_transaction(
        &mut tx,
        account.account_id,
        Some(req.from_account_id),
        Some(req.to_account_id),
        TransactionType::Transfer,
        req.amount,
        req.reference_id.clone(),
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    match DbOperations::update_balance(
        &mut tx,
        req.from_account_id,
        from_balance - req.amount,
    )
        .await
    {
        Ok(_) => {
            match DbOperations::update_balance(
                &mut tx,
                req.to_account_id,
                to_balance + req.amount,
            )
                .await
            {
                Ok(_) => {
                    match DbOperations::mark_transaction_status(
                        &mut tx,
                        txn_id,
                        TransactionStatus::Succeeded,
                    )
                        .await
                    {
                        Ok(_) => {

                            // -------- WEBHOOK EVENT LOGIC --------

                            let payload = serde_json::json!({
                                "event": "transaction.succeeded",
                                "data": {
                                    "transaction_id": txn_id,
                                    "type": "transfer",
                                    "amount": req.amount,
                                    "from_account_id": req.from_account_id,
                                    "to_account_id": req.to_account_id,
                                    "business_id": account.account_id,
                                    "reference_id": req.reference_id
                                }
                            });

                            let webhook_id = match app_state.database_connector.get_webhook(account.account_id).await  {
                                Ok(webhook_id) => webhook_id,
                                Err(err) => {
                                    tracing::error!("no webhook was registered to this business account") ;
                                    tracing::error!("error was {}", err) ;
                                    tx.rollback().await.ok();
                                    return (
                                        axum::http::StatusCode::NOT_FOUND,
                                        Json(ApiResponse::<i64>::error("Register a Webhook First".to_string())),
                                    );
                                }
                            };

                            let webhook_event_id = match app_state
                                .database_connector
                                .create_webhook_event(
                                    /* webhook_id */ webhook_id,
                                    "transaction.succeeded",
                                    payload,
                                )
                                .await
                            {
                                Ok(id) => id,
                                Err(e) => {
                                    tracing::error!("failed to create webhook event {}", e);
                                    tx.rollback().await.ok();
                                    return (
                                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                        Json(ApiResponse::<i64>::error(e.to_string())),
                                    );
                                }
                            };

                            // -----------------------------------

                            tx.commit().await.unwrap();

                            app_state
                                .event_queue
                                .send(WebhookQueueMessage { webhook_event_id, webhook_id })
                                .expect("Unable to add Event Id to the Queue");

                            (
                                axum::http::StatusCode::CREATED,
                                Json(ApiResponse::success(txn_id)),
                            )
                        }
                        Err(err) => {
                            tracing::error!(
                                "Failed to mark transaction status: {:?}",
                                err
                            );
                            tx.rollback().await.ok();
                            return (
                                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<i64>::error(err.to_string())),
                            );
                        }
                    }
                }
                Err(err) => {
                    tx.rollback().await.ok();
                    return (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<i64>::error(err.to_string())),
                    );
                }
            }
        }
        Err(err) => {
            tx.rollback().await.ok();
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<i64>::error(err.to_string())),
            );
        }
    }
}




pub async fn get_all_transactions(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
) -> impl IntoResponse {

    let rows = sqlx::query(
        "SELECT id, business_id, from_account_id, to_account_id,
                type::TEXT, amount, status::TEXT, reference_id,
                idempotency_key, created_at
         FROM transactions
         WHERE business_id = $1
         ORDER BY created_at DESC"
    )
        .bind(account.account_id)
        .fetch_all(&app_state.database_connector.connector)
        .await;

    match rows {
        Ok(rows) => {
            let txns = rows
                .into_iter()
                .map(|r| Transaction {
                    id: r.get("id"),
                    business_id: r.get("business_id"),
                    from_account_id: r.get("from_account_id"),
                    to_account_id: r.get("to_account_id"),
                    txn_type: r.get("type"),
                    amount: r.get("amount"),
                    status: r.get("status"),
                    reference_id: r.get("reference_id"),
                    idempotency_key: r.get("idempotency_key"),
                    created_at: r.get("created_at"),
                })
                .collect::<Vec<_>>();

            (
                axum::http::StatusCode::OK,
                Json(ApiResponse::success(txns)),
            )
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<Transaction>>::error(e.to_string())),
        ),
    }
}



pub async fn get_transaction_details(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Path(transaction_id): Path<i64>,
) -> impl IntoResponse {

    let row = sqlx::query(
        "SELECT id, business_id, from_account_id, to_account_id,
                type::TEXT, amount, status::TEXT, reference_id,
                idempotency_key, created_at
         FROM transactions
         WHERE id = $1 AND business_id = $2"
    )
        .bind(transaction_id)
        .bind(account.account_id)
        .fetch_optional(&app_state.database_connector.connector)
        .await;

    match row {
        Ok(Some(r)) => (
            axum::http::StatusCode::OK,
            Json(ApiResponse::success(Transaction {
                id: r.get("id"),
                business_id: r.get("business_id"),
                from_account_id: r.get("from_account_id"),
                to_account_id: r.get("to_account_id"),
                txn_type: r.get("type"),
                amount: r.get("amount"),
                status: r.get("status"),
                reference_id: r.get("reference_id"),
                idempotency_key: r.get("idempotency_key"),
                created_at: r.get("created_at"),
            })),
        ),
        Ok(None) => (
            axum::http::StatusCode::NOT_FOUND,
            Json(ApiResponse::<Transaction>::error("Transaction not found".into())),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Transaction>::error(e.to_string())),
        ),
    }
}

