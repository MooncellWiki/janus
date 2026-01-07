use anyhow::Result;
use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use tracing::info;

#[async_trait]
pub trait Repository: Send + Sync + Clone + 'static {
    async fn health_check(&self) -> bool;
    async fn migrate(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct PostgresRepository {
    pub pool: Pool<Postgres>,
}

#[async_trait]
impl Repository for PostgresRepository {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").execute(&self.pool).await.is_ok()
    }

    async fn migrate(&self) -> Result<()> {
        info!("Running database migrations");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        info!("Database migrations completed");
        Ok(())
    }
}
