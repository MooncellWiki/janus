use crate::{
    config::{AliyunConfig, AppSettings, BilibiliConfig, JwtConfig},
    repository::PostgresRepository,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub bilibili_config: BilibiliConfig,
    pub jwt_config: JwtConfig,
    pub aliyun_config: AliyunConfig,
    pub http_client: reqwest::Client,
}

pub async fn init_state_with_pg(config: &AppSettings) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database.uri)
        .await
        .expect("Failed to connect to the database");

    AppState {
        repository: PostgresRepository { pool },
        bilibili_config: config.bilibili.clone(),
        jwt_config: config.jwt.clone(),
        aliyun_config: config.aliyun.clone(),
        http_client: reqwest::Client::new(),
    }
}
