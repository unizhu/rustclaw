mod config;
mod service;

use anyhow::Result;
use config::Config;
use service::GatewayService;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::load()?;

    // Create and run gateway service
    let gateway = GatewayService::new(config);
    gateway.run().await
}
