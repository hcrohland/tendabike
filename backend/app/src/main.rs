#![warn(clippy::all)]

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let db = tb_diesel::DbPool::new().await?;
    let path = get_static_path();
    let socket = get_socket_address();

    tb_axum::start(db, path, socket).await;

    Ok(())
}

fn get_static_path() -> PathBuf {
    let path = std::env::var("STATIC_WWW").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/dist").to_string()
    });

    Path::new(&path)
        .canonicalize()
        .unwrap_or_else(|_| panic!("STATIC_WWW Path {path} does not exist"))
}

fn get_socket_address() -> SocketAddr {
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    addr.parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("BIND_ADDR '{addr}' could not be parsed"))
}
