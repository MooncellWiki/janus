// CLI main entry point
use anyhow::Result;
use janus::app::run;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments and run
    run().await?;

    Ok(())
}
