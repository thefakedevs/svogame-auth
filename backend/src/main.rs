mod app;
mod domains;
mod entities;
#[path = "domains/skins/png_checker/mod.rs"]
mod png_checker;
mod services;
mod util;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    app::bootstrap::run().await
}
