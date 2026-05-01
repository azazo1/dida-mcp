mod config;
mod proxy;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    server::run().await
}
