use crate::{
    aliyun::CdnClient,
    config::{AliyunConfig, AppSettings, BilibiliConfig, JwtConfig},
    repository::PostgresRepository,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub repository: PostgresRepository,
    pub bilibili_config: BilibiliConfig,
    pub jwt_config: JwtConfig,
    pub http_client: reqwest::Client,
    pub aliyun_client: Option<CdnClient>,
}

pub async fn init_state_with_pg(config: &AppSettings) -> AppState {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&config.database.uri)
        .await
        .expect("Failed to connect to the database");

    let http_client = reqwest::Client::new();

    // Initialize Alibaba Cloud CDN client if config is present
    let aliyun_client = config.aliyun.as_ref().map(|aliyun_config: &AliyunConfig| {
        CdnClient::with_client(
            aliyun_config.access_key_id.clone(),
            aliyun_config.access_key_secret.clone(),
            aliyun_config.cdn_endpoint.clone(),
            http_client.clone(),
        )
    });

    AppState {
        repository: PostgresRepository { pool },
        bilibili_config: config.bilibili.clone(),
        jwt_config: config.jwt.clone(),
        http_client,
        aliyun_client,
    }
}
