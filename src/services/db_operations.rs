use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;

pub struct DbOperations {
    connector: Pool<Postgres>
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
}