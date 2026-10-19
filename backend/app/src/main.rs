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
    let db = pool.raw();

    // Build the `Registry` over the pool (it owns the seam — the web layer
    // names the trait, never a seam type, spec #558 §1/§6.1) and hand the
    // web layer the service API handle. The raw pool handed to the tower
    // session store is unchanged (it is not a seam type).
    let api = std::sync::Arc::new(tb_exec::Registry::with_defaults(pool))
        as std::sync::Arc<dyn tb_exec::ServiceApi>;

    Ok(tb_axum::start(api, db, path, addr).await?)
}
