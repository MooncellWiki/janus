// CLI main entry point
use anyhow::Result;
use mimalloc::MiMalloc;
use my_axum_template::app::run;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments and run
    run().await?;

    Ok(())
}
