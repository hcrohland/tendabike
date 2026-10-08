#![warn(clippy::all)]

use std::{net::SocketAddr, path::Path};

use mimalloc::MiMalloc;
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://localhost/tendabike".to_string());

    let path = std::env::var("STATIC_WWW").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/dist").to_string()
    });
    let path = Path::new(&path)
        .canonicalize()
        .unwrap_or_else(|_| panic!("STATIC_WWW Path {path} does not exist"));

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
    let addr = addr
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("BIND_ADDR '{addr}' could not be parsed"));

    // The logging subscriber moved here from `tb_axum::start` with the web
    // layer becoming storage-agnostic: the composition root owns process setup.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = tb_sqlx::DbPool::new(&database_url).await?;
    let session_store = PostgresStore::new(pool.raw());

    Ok(tb_axum::start(pool, session_store, path, addr).await?)
}
