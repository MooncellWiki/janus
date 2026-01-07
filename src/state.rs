use crate::{
    config::{AppSettings, BilibiliConfig},
    repository::PostgresRepository,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub bilibili_config: Option<BilibiliConfig>,
}

pub async fn init_state_with_pg(config: &AppSettings) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database.uri)
        .await
        .expect("Failed to connect to the database");

    AppState {
        repository: PostgresRepository { pool },
        bilibili_config: config.bilibili.clone(),
    }
}
