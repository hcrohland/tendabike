#![warn(clippy::all)]

use std::{net::SocketAddr, path::Path};

use mimalloc::MiMalloc;

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

    // The composition root wires the concrete crates and process setup;
    // the crate-internal details (the session store, the logging
    // subscriber) live in `tb_axum::start`.
    let pool = tb_sqlx::DbPool::new(&database_url).await?;

    Ok(tb_axum::start(pool.clone(), pool.raw(), path, addr).await?)
}
