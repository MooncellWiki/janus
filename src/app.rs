use anyhow::Result;
use clap::Parser;
use std::path::Path;
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    auth::generate_token,
    config::AppSettings,
    routes::build_router,
    shutdown::shutdown_signal,
    state::init_state,
    tracing::{init_sentry, init_tracing},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub enum Commands {
    /// Start the web server
    Server {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
    },
    /// Generate a JWT token
    GenerateJwt {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Subject for the JWT (e.g., user ID or identifier)
        #[arg(short, long)]
        subject: String,
    },
    /// Refresh CDN cache for an object
    RefreshCdn {
        #[arg(short, long, default_value = "config.toml")]
        config: String,
        /// Object key in the bucket
        #[arg(short, long)]
        object_key: String,
        /// Bucket name (must exist in bucket_url_map config)
        #[arg(short, long)]
        bucket_name: String,
    },
    /// Show version information
    Version,
}

async fn start(config: &AppSettings) -> Result<()> {
    // // Build router
    let listener = TcpListener::bind(config.server.full_url()).await?;
    info!("Server is running on {}", config.server.full_url());
    let state = init_state(config).await;
    let router = build_router(state);
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Web server has gracefully shutdown");
    Ok(())
}

pub async fn run() -> Result<()> {
    let cli = Commands::parse();
    match cli {
        Commands::Server { config } => {
            let config = AppSettings::new(Path::new(&config))?;

            init_tracing(&config.logger);
            let _sentry_guard = &config.sentry.as_ref().map(init_sentry);
            start(&config).await?;
            Ok(())
        }
        Commands::GenerateJwt { config, subject } => {
            let config = AppSettings::new(Path::new(&config))?;

            let token = generate_token(subject.clone(), &config.jwt.private_key)?;

            println!("Generated JWT token for subject '{}':", subject);
            println!("{}", token);

            Ok(())
        }
        Commands::RefreshCdn {
            config,
            object_key,
            bucket_name,
        } => {
            let config = AppSettings::new(Path::new(&config))?;

            let url_template = config
                .aliyun
                .bucket_url_map
                .get(&bucket_name)
                .ok_or_else(|| anyhow::anyhow!("Unsupported bucket: {}", bucket_name))?;

            use percent_encoding::percent_encode;
            let encoded_object_key =
                percent_encode(object_key.as_bytes(), crate::routes::URI).to_string();
            let object_url = url_template.replace("{object_key}", &encoded_object_key);

            let http_client = reqwest::Client::new();
            let client = crate::aliyun::AliyunCdnClient::new(&config.aliyun, http_client);

            let request = crate::aliyun::RefreshObjectCachesRequest {
                object_path: object_url.clone(),
                object_type: Some("File".to_string()),
                force: Some(false),
            };

            let response = client.refresh_object_caches(&request).await?;

            println!(
                "CDN refresh triggered for '{}' in bucket '{}'",
                object_key, bucket_name
            );
            println!("Task ID: {}", response.refresh_task_id);

            Ok(())
        }
        Commands::Version => {
            println!(
                "{} ({})",
                env!("CARGO_PKG_VERSION"),
                option_env!("BUILD_SHA")
                    .or(option_env!("GITHUB_SHA"))
                    .unwrap_or("dev")
            );
            Ok(())
        }
    }
}
