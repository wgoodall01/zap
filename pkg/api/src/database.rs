use anyhow::Result;
use sqlx::{PgPool, Pool, Postgres};

pub type DbPool = Pool<Postgres>;

pub async fn create_pool(database_url: &str) -> Result<DbPool> {
    let pool = PgPool::connect(database_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
