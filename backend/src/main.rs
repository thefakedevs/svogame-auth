mod app;
mod domains;
mod entities;
mod services;
mod util;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    app::bootstrap::run().await
}
