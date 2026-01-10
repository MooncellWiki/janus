use anyhow::Result;
use clap::Parser;
use std::path::Path;
use tokio::net::TcpListener;
use tracing::info;

use crate::{
    auth::generate_token,
    config::AppSettings,
    repository::Repository,
    routes::build_router,
    shutdown::shutdown_signal,
    state::init_state_with_pg,
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
    /// Show version information
    Version,
}

async fn start(config: &AppSettings) -> Result<()> {
    // // Build router
    let listener = TcpListener::bind(config.server.full_url()).await?;
    info!("Server is running on {}", config.server.full_url());
    let state = init_state_with_pg(config).await;
    state.repository.migrate().await?;
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
