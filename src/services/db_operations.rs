use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{Pool, Postgres, Row, Transaction};
use sqlx::postgres::PgPoolOptions;
use crate::models;
use crate::models::accounts_models::{Account, NewAccount};
use crate::models::bussiness_models::BusinessState;
use crate::models::transaction_models::{TransactionStatus, TransactionType};
use crate::models::webhooks_models::{WebhookEventRow, WebhookResponse, WebhookRow};

pub struct DbOperations {
    pub(crate) connector: Pool<Postgres>
}

impl DbOperations {
    pub async fn new() -> DbOperations {
        let host = std::env::var("HOST").unwrap();
        let username = std::env::var("USERNAME").unwrap();
        let password = std::env::var("PASSWORD").unwrap();
        let db_name = std::env::var("DB_NAME").unwrap();
        let url = format!("postgres://{}:{}@{}/{}", username, password, host, db_name);
        let max_connections = std::env::var("MAX_CONNECTIONS").unwrap().parse::<u32>().unwrap();
        let pool = PgPoolOptions::new().max_connections(max_connections).connect(&url).await.unwrap() ;
        
        tracing::info!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
        tracing::info!("Database migrations complete.");

        DbOperations {
            connector: pool
        }
    }

    pub async fn get_admin_count(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query("select count(*) from admins").fetch_one(&self.connector).await ;
        match result {
            Ok(res) => Ok(res.get(0)),
            Err(err) => Err(err)
        }
    }

    pub async fn create_new_business(&self, name: &str) -> Result<i64, sqlx::Error> {
        // we need to return the response as business id
        let result = sqlx::query("insert into businesses (name) values ($1) returning id")
            .bind(name)
            .fetch_one(&self.connector).await ;
        tracing::info!("executed the create new bussiness query") ;
        match result {
            Ok(res) => {
                tracing::info!("got the id after creating business") ;
                let id: i64 = res.get("id") ;
                Ok(id)
            },
            Err(err) => {
                tracing::error!("occurred while inserting a new business {}", err) ;
                Err(err)
            }
        }
    }


    pub async fn get_businesses(&self) -> Result<Vec<BusinessState>, sqlx::Error> {
        let result = sqlx::query("select id, name, status::Text, created_at from businesses ORDER BY created_at DESC").fetch_all(&self.connector).await ;
        tracing::info!("executed query to get all businesses") ;
        match result {
            Ok(res) => Ok(res.into_iter().map(|row| BusinessState { id: row.get("id"), name: row.get("name"), status: row.get("status"), created_at: row.get("created_at") }).collect()),
            Err(err) => {
                tracing::error!("occurred while getting all businesses {}", err) ;
                Err(err)
            }
        }
    }


    pub async fn get_admin_id(&self, name: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query("select id from admins where username = $1")
            .bind(name)
            .fetch_one(&self.connector).await ;
        match result {
            Ok(result) => {
                tracing::info!("got the admin id") ;
                Ok(result.get("id"))
            },
            Err(err) => {
                tracing::error!("got the error while getting admin id, {}", err) ;
                Err(err)
            }
        }
    }

    pub async fn create_admin_account(&self, name: &str) -> Result<i64, sqlx::Error> {

        match self.get_admin_count().await {
            Ok(count) => {
                if count == 0 {
                    tracing::info!("no admins exists creating default one") ;
                    tracing::info!("Creating admin account with name {}", name) ;
                    let result = sqlx::query("insert into admins (username) values ($1) returning id")
                        .bind(name)
                        .fetch_one(&self.connector).await ;
                    match result {
                        Ok(result) => {
                            tracing::info!("successfully inserted new admin") ;
                            Ok(result.get("id"))
                        },
                        Err(err) => {
                            tracing::error!("error while creating an admin {}", err) ;
                            Err(err)
                        }
                    }
                }else{
                    tracing::info!("the default user exists getting id") ;
                    match self.get_admin_id(name).await {
                        Ok(id) => {
                            Ok(id)
                        },
                        Err(err) => {
                            tracing::warn!("probably the username doesn't exists in db") ;
                            Err(err)
                        }
                    }
                }
            },
            Err(err) => {
                tracing::error!("we got the error while getting admin count {}", err) ;
                Err(err)
            }
        }

    }

    pub async fn validate_business_id(&self, business_id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("select status::Text from businesses where id = $1")
        .bind(business_id)
            .fetch_one(&self.connector).await ;

        match result {
            Ok(result) => {
                let status: String = result.get("status") ;
                if status == "active" {
                    return Ok(true)
                }
                Ok(false)
            },
            Err(err) => {
                tracing::error!("error occurred while validating business id {}", err) ;
                Err(err)
            }
        }
    }


    pub async fn store_api_key(&self, business_id: i64, key_hash: &str) -> Result<(), sqlx::Error> {
        let result = sqlx::query("insert into api_keys (business_id, key_hash) values ($1, $2)")
            .bind(business_id) .bind(key_hash) .execute(&self.connector).await ;
        tracing::info!("executed an insert query for storing the new api key") ;
        match result {
            Ok(res) => {
                tracing::info!("successfully inserted") ;
                Ok(())
            },
            Err(err) => {
                tracing::error!("got an error {}", err) ;
                Err(err)
            }
        }
    }


    pub async fn store_api_key_txn(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        business_id: i64,
        key_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO api_keys (business_id, key_hash, status)
             VALUES ($1, $2, 'active')"
        )
            .bind(business_id)
            .bind(key_hash)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn expire_api_key_txn(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key_id: i64,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE api_keys
             SET status = 'expiring',
                 expires_at = $1
             WHERE id = $2"
        )
            .bind(expires_at)
            .bind(key_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    pub async fn revoke_api_key_txn(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        key_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE api_keys
             SET status = 'revoked',
                 expires_at = NOW()
             WHERE id = $1",
        )
            .bind(key_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    // ===============================
    // ADMIN API KEYS
    // ===============================

    pub async fn store_admin_api_key(
        &self,
        admin_id: i64,
        key_hash: &str,
    ) -> Result<(), sqlx::Error> {
        tracing::info!("executing insert query for new admin api key") ;
        sqlx::query(
            "INSERT INTO admin_api_keys (admin_id, key_hash)
             VALUES ($1, $2)"
        )
            .bind(admin_id)
            .bind(key_hash)
            .execute(&self.connector)
            .await?;
        Ok(())
    }

    pub async fn revoke_admin_api_key(
        &self,
        key_id: i64,
    ) -> Result<(), sqlx::Error> {
        tracing::info!("executing update query changing status to revoked for key : {}", key_id) ;
        sqlx::query(
            "UPDATE admin_api_keys
             SET status = 'revoked'
             WHERE id = $1",
        )
            .bind(key_id)
            .execute(&self.connector)
            .await?;
        Ok(())
    }


    pub async fn get_accounts_by_business(
        &self,
        business_id: i64,
    ) -> Result<Vec<Account>, sqlx::Error> {

        let rows = sqlx::query(
        r#"
        SELECT id, name, currency, status::TEXT, balance, created_at
        FROM business_accounts
        WHERE business_id = $1
        ORDER BY created_at DESC
        "#
    )
            .bind(business_id)
            .fetch_all(&self.connector)
            .await?;
        tracing::info!("executed query for getting all the accounts for the business") ;
        Ok(rows.into_iter().map(|r| Account {
            id: r.get("id"),
            name: r.get("name"),
            currency: r.get("currency"),
            status: r.get("status"),
            balance: r.get("balance"),
            created_at: r.get("created_at"),
        }).collect())
    }


    pub async fn validate_account_ownership(
        &self,
        business_id: i64,
        account_id: i64,
    ) -> Result<bool, sqlx::Error> {

        let res = sqlx::query(
        "SELECT 1 FROM business_accounts WHERE id = $1 AND business_id = $2"
    )
            .bind(account_id)
            .bind(business_id)
            .fetch_optional(&self.connector)
            .await?;

        Ok(res.is_some())
    }

    pub async fn get_account_balance(
        &self,
        account_id: i64,
    ) -> Result<Decimal, sqlx::Error> {

        let row = sqlx::query(
        "SELECT balance FROM business_accounts WHERE id = $1"
    )
            .bind(account_id)
            .fetch_one(&self.connector)
            .await?;
        tracing::info!("executed query for getting account balance") ;
        Ok(row.get("balance"))
    }

    pub async fn get_account_details(
        &self,
        account_id: i64,
    ) -> Result<Account, sqlx::Error> {

        let r = sqlx::query(
        r#"
        SELECT id, name, currency, status::TEXT, balance, created_at
        FROM business_accounts
        WHERE id = $1
        "#
    )       .bind(account_id)
            .fetch_one(&self.connector)
            .await?;
    tracing::info!("executed query for getting account details") ;
        Ok(Account {
            id: r.get("id"),
            name: r.get("name"),
            currency: r.get("currency"),
            status: r.get("status"),
            balance: r.get("balance"),
            created_at: r.get("created_at"),
        })
    }


    pub async fn create_account(
        &self,
        business_id: i64,
        new_account: NewAccount,
    ) -> Result<i64, sqlx::Error> {

        let row = sqlx::query(
        r#"
        INSERT INTO business_accounts (business_id, name, currency)
        VALUES ($1, $2, $3)
        RETURNING id
        "#
    )
            .bind(business_id)
            .bind(new_account.name)
            .bind(new_account.currency)
            .fetch_one(&self.connector)
            .await?;
        tracing::info!("executed the creation of business account sucessfully") ;
        Ok(row.get("id"))
    }

    pub async fn verify_business_api_key(
        &self,
        key_hash: &str,
    ) -> Result<i64, sqlx::Error> {
        tracing::info!("the key hash was {}", key_hash) ;
        let rec = sqlx::query(
        r#"
        SELECT business_id
        FROM api_keys
        WHERE key_hash = $1
          AND status = 'active'
          AND (expires_at IS NULL OR expires_at > now())
        "#
        
    )       .bind(key_hash)
            .fetch_optional(&self.connector)
            .await?;

        match rec {
            Some(r) => Ok(r.get("business_id")),
            None => Err(sqlx::Error::RowNotFound),
        }
    }


    pub async fn verify_admin_api_key(
        &self,
        key_hash: &str,
    ) -> Result<i64, sqlx::Error> {

        let rec = sqlx::query(
        r#"
        SELECT admin_id
        FROM admin_api_keys
        WHERE key_hash = $1
          AND status = 'active'
        "#
    )       .bind(key_hash)
            .fetch_optional(&self.connector)
            .await?;

        match rec {
            Some(r) => Ok(r.get("admin_id")),
            None => Err(sqlx::Error::RowNotFound),
        }
    }

    pub async fn get_business_account_by_id(&self, account_id: i64) -> Result<BusinessState, sqlx::Error> {
        let row = sqlx::query(
        "
        SELECT id, name,status::TEXT, created_at
        FROM businesses
        WHERE id = $1
        " ).bind(account_id).fetch_one(&self.connector).await ;

        match row {
            Ok(res) => {
                tracing::info!("got the business account") ;
                Ok(BusinessState {
                    id: res.get("id"),
                    name: res.get("name"),
                    status: res.get("status"),
                    created_at: res.get("created_at")
                })
            },
            Err(err) => {
                tracing::error!("the error was {}", err) ;
                Err(err)
            }
        }
    }

    pub async fn lock_account(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account_id: i64,
    ) -> Result<(Decimal, String), sqlx::Error> {

        let row = sqlx::query(
            "SELECT balance, status::TEXT
         FROM business_accounts
         WHERE id = $1
         FOR UPDATE"
        )
            .bind(account_id)
            .fetch_one(&mut **tx)
            .await?;

        Ok((row.get("balance"), row.get("status")))
    }


    pub async fn check_idempotency(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        business_id: i64,
        key: &str,
    ) -> Result<Option<i64>, sqlx::Error> {

        let row = sqlx::query(
            "SELECT id FROM transactions
         WHERE business_id = $1 AND idempotency_key = $2"
        )
            .bind(business_id)
            .bind(key)
            .fetch_optional(&mut **tx)
            .await?;

        Ok(row.map(|r| r.get("id")))
    }


    pub async fn insert_transaction(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        business_id: i64,
        from_account: Option<i64>,
        to_account: Option<i64>,
        txn_type: TransactionType,
        amount: Decimal,
        reference_id: Option<String>,
        idempotency_key: &str,
        status: TransactionStatus,
    ) -> Result<i64, sqlx::Error> {

        let row = sqlx::query(
            "INSERT INTO transactions
         (business_id, from_account_id, to_account_id, type, amount, status, reference_id, idempotency_key)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         RETURNING id"
        )
            .bind(business_id)
            .bind(from_account)
            .bind(to_account)
            .bind(txn_type)
            .bind(amount)
            .bind(status)
            .bind(reference_id)
            .bind(idempotency_key)
            .fetch_one(&mut **tx)
            .await?;

        Ok(row.get("id"))
    }


    pub async fn update_balance(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account_id: i64,
        new_balance: Decimal,
    ) -> Result<(), sqlx::Error> {

        sqlx::query(
            "UPDATE business_accounts
         SET balance = $1
         WHERE id = $2"
        )
            .bind(new_balance)
            .bind(account_id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    pub async fn mark_transaction_status(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        txn_id: i64,
        status: TransactionStatus,
    ) -> Result<(), sqlx::Error> {

        sqlx::query(
            "UPDATE transactions SET status = $1 WHERE id = $2"
        )
            .bind(status)
            .bind(txn_id)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }



    pub async fn get_webhooks_by_business(
        &self,
        business_id: i64,
    ) -> Result<Vec<WebhookResponse>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
        SELECT id, url, business_id, status, created_at
        FROM webhooks
        WHERE business_id = $1
        ORDER BY created_at DESC
        "#
        )
            .bind(business_id)
            .fetch_all(&self.connector)
            .await?;

        let webhooks = rows
            .into_iter()
            .map(|row| WebhookResponse {
                id: row.get("id"),
                url: row.get("url"),
                business_id: row.get("business_id"),
                status: row.get("status"),
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(webhooks)
    }


    pub async fn create_webhook(
        &self,
        business_id: i64,
        url: &str,
    ) -> Result<i64, sqlx::Error> {
        let row = sqlx::query(
            r#"
        INSERT INTO webhooks (business_id, url, secret, status)
        VALUES ($1, $2, gen_random_uuid()::text, 'active')
        RETURNING id
        "#
        )
            .bind(business_id)
            .bind(url)
            .fetch_one(&self.connector)
            .await?;

        Ok(row.get("id"))
    }

    pub async fn disable_webhook(
        &self,
        business_id: i64,
        webhook_id: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        UPDATE webhooks
        SET status = 'disabled'
        WHERE id = $1 AND business_id = $2
        "#
        )
            .bind(webhook_id)
            .bind(business_id)
            .execute(&self.connector)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn update_webhook(
        &self,
        business_id: i64,
        webhook_id: i64,
        url: Option<String>,
        status: Option<String>,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        UPDATE webhooks
        SET
            url = COALESCE($1, url),
            status = COALESCE($2, status)
        WHERE id = $3 AND business_id = $4
        "#
        )
            .bind(url)
            .bind(status)
            .bind(webhook_id)
            .bind(business_id)
            .execute(&self.connector)
            .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_webhook(&self, business_id: i64) -> Result<i64, sqlx::Error> {
        let row = sqlx::query("select id from webhooks where business_id = $1 AND status = 'active' ")
            .bind(business_id)
            .fetch_one(&self.connector).await? ;
        Ok(row.get("id"))
    }
    
    // get_full_webhook
    pub async fn get_full_webhook(&self, id: i64) -> Result<WebhookRow, sqlx::Error> {
        let row = sqlx::query("select id, business_id, url, status::Text, secret from webhooks where id=$1 ")
            .bind(id)
            .fetch_one(&self.connector).await? ;
        Ok(WebhookRow {
            id: row.get("id"),
            business_id: row.get("business_id"),
            status: row.get("status"),
            url: row.get("url"),
            secret: row.get("secret")
        })
    }

    pub async fn create_webhook_event(
        &self,
        webhook_id: i64,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<i64, sqlx::Error> {
        let row = sqlx::query(
            r#"
        INSERT INTO webhook_events (
            webhook_id,
            event_type,
            payload,
            status,
            attempt_count
        )
        VALUES ($1, $2, $3, 'pending', 0)
        RETURNING id
        "#
        )
            .bind(webhook_id)
            .bind(event_type)
            .bind(payload)
            .fetch_one(&self.connector)
            .await?;

        Ok(row.get("id"))
    }


    pub async fn get_webhook_event(
        &self,
        event_id: i64,
    ) -> Result<Option<WebhookEventRow>, sqlx::Error> {
        let row = sqlx::query(
            r#"
        SELECT
            id,
            webhook_id,
            event_type,
            payload,
            status::Text,
            attempt_count,
            next_retry_at,
            created_at
        FROM webhook_events
        WHERE id = $1
        "#
        )
            .bind(event_id)
            .fetch_optional(&self.connector)
            .await?;

        Ok(row.map(|r| WebhookEventRow {
            id: r.get("id"),
            webhook_id: r.get("webhook_id"),
            event_type: r.get("event_type"),
            payload: r.get("payload"),
            status: r.get("status"),
            attempt_count: r.get("attempt_count"),
            next_retry_at: r.get("next_retry_at"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn mark_webhook_event_delivered(
        &self,
        event_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE webhook_events
        SET status = 'delivered'
        WHERE id = $1
        "#
        )
            .bind(event_id)
            .execute(&self.connector)
            .await?;

        Ok(())
    }


    pub async fn schedule_webhook_retry(
        &self,
        event_id: i64,
        next_retry_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE webhook_events
        SET
            attempt_count = attempt_count + 1,
            next_retry_at = $2
        WHERE id = $1
          AND status = 'pending'
        "#
        )
            .bind(event_id)
            .bind(next_retry_at)
            .execute(&self.connector)
            .await?;

        Ok(())
    }


    pub async fn mark_webhook_event_failed(
        &self,
        event_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE webhook_events
        SET status = 'failed'
        WHERE id = $1
        "#
        )
            .bind(event_id)
            .execute(&self.connector)
            .await?;

        Ok(())
    }


    pub async fn get_pending_webhook_events(
        &self,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
        SELECT id
        FROM webhook_events
        WHERE status = 'pending'
          AND (next_retry_at IS NULL OR next_retry_at <= now())
        ORDER BY created_at
        "#
        )
            .fetch_all(&self.connector)
            .await?;

        Ok(rows.into_iter().map(|r| r.get("id")).collect())
    }


}