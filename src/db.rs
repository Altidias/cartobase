use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(10))
        .connect(database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
