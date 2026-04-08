use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    auth::app::bootstrap::run().await
}
