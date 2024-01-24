use cluster_api::controller;

use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub async fn init() {
    // Setup tracing layers
    let logger = tracing_subscriber::fmt::layer().compact();
    let env_filter = EnvFilter::try_from_default_env()
        .or(EnvFilter::try_new("info"))
        .unwrap();

    let collector = Registry::default().with(logger).with(env_filter);

    // Initialize tracing
    tracing::subscriber::set_global_default(collector).unwrap();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init().await;
    // Initiatilize Kubernetes controller state
    let controller = controller::run();

    tokio::join!(controller).0;
    Ok(())
}
