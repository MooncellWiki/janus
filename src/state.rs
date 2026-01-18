use crate::config::{AliyunConfig, AppSettings, BilibiliConfig, JwtConfig};

#[derive(Debug, Clone)]
pub struct AppState {
    pub bilibili_config: BilibiliConfig,
    pub jwt_config: JwtConfig,
    pub aliyun_config: AliyunConfig,
    pub http_client: reqwest::Client,
}

pub async fn init_state(config: &AppSettings) -> AppState {
    AppState {
        bilibili_config: config.bilibili.clone(),
        jwt_config: config.jwt.clone(),
        aliyun_config: config.aliyun.clone(),
        http_client: reqwest::Client::new(),
    }
}
