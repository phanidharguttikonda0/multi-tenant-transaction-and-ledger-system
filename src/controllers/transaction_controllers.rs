use std::sync::Arc;
use axum::{Extension, Json};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use sqlx::Row;
use crate::AppState;
use crate::models::common::{AccountId, ApiResponse};
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
        req.reference_id,
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    DbOperations::update_balance(&mut tx, req.to_account_id, balance + req.amount)
        .await
        .unwrap();

    DbOperations::mark_transaction_status(&mut tx, txn_id, TransactionStatus::Succeeded)
        .await
        .unwrap();

    tx.commit().await.unwrap();

    (
        axum::http::StatusCode::CREATED,
        Json(ApiResponse::success(txn_id)),
    )
}



pub async fn debit_money(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Json(req): Json<DebitRequest>,
) -> impl IntoResponse {

    let mut tx = app_state.database_connector.connector.begin().await.unwrap();

    if let Some(txn_id) =
        DbOperations::check_idempotency(&mut tx, account.account_id, &req.idempotency_key)
            .await
            .unwrap()
    {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::OK,
            Json(ApiResponse::<i64>::error("Balance is zero".to_string())),
        );
    }

    let (balance, status) =
        DbOperations::lock_account(&mut tx, req.from_account_id).await.unwrap();

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
        req.reference_id,
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    DbOperations::update_balance(&mut tx, req.from_account_id, balance - req.amount)
        .await
        .unwrap();
    
    DbOperations::mark_transaction_status(&mut tx, txn_id, TransactionStatus::Succeeded)
        .await
        .unwrap();

    tx.commit().await.unwrap();

    (
        axum::http::StatusCode::CREATED,
        Json(ApiResponse::success(txn_id)),
    )
}



pub async fn transfer_money(
    State(app_state): State<Arc<AppState>>,
    Extension(account): Extension<AccountId>,
    Json(req): Json<TransferRequest>,
) -> impl IntoResponse {

    let mut tx = app_state.database_connector.connector.begin().await.unwrap();

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

    let (from_balance, _) =
        DbOperations::lock_account(&mut tx, req.from_account_id).await.unwrap();
    let (to_balance, _) =
        DbOperations::lock_account(&mut tx, req.to_account_id).await.unwrap();

    if from_balance < req.amount {
        tx.rollback().await.ok();
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(ApiResponse::<i64>::error("Insufficient balance".into())),
        );
    }

    let txn_id = DbOperations::insert_transaction(
        &mut tx,
        account.account_id,
        Some(req.from_account_id),
        Some(req.to_account_id),
        TransactionType::Transfer,
        req.amount,
        req.reference_id,
        &req.idempotency_key,
        TransactionStatus::Pending,
    )
        .await
        .unwrap();

    DbOperations::update_balance(&mut tx, req.from_account_id, from_balance - req.amount)
        .await
        .unwrap();
    DbOperations::update_balance(&mut tx, req.to_account_id, to_balance + req.amount)
        .await
        .unwrap();

    DbOperations::mark_transaction_status(&mut tx, txn_id, TransactionStatus::Succeeded)
        .await
        .unwrap();

    tx.commit().await.unwrap();

    (
        axum::http::StatusCode::CREATED,
        Json(ApiResponse::success(txn_id)),
    )
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

