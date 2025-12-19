use rust_decimal::Decimal;
use sqlx::{Pool, Postgres, Row, Transaction};
use sqlx::postgres::PgPoolOptions;
use crate::models::accounts_models::{Account, NewAccount};
use crate::models::bussiness_models::BusinessState;

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
        match result {
            Ok(res) => Ok(res.into_iter().map(|row| BusinessState { id: row.get("id"), name: row.get("name"), status: row.get("status"), created_at: row.get("created_at") }).collect()),
            Err(err) => Err(err)
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

        Ok(row.get("id"))
    }

    pub async fn verify_business_api_key(
        &self,
        key_hash: &str,
    ) -> Result<i64, sqlx::Error> {

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
        txn_type: &str,
        amount: Decimal,
        reference_id: Option<String>,
        idempotency_key: &str,
        status: &str,
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
        status: &str,
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

}