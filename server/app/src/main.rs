#![warn(clippy::all)]

use kernel::s_diesel::DbPool;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

fn main() -> anyhow::Result<()> {
    // setup environment. Includes Config and logging
    init_environment();

    let db = DbPool::init()?;
    let path = get_static_path();
    let socket = get_socket_address();

    // #[cfg(axum)]
    tb_axum::start(db, path, socket);
    // #[cfg(not(axum))]
    // tb_rocket::start(DbPool(db), path);

    Ok(())
}

fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    // env_logger::Builder::from_env(
    //     env_logger::Env::default().default_filter_or("tendabike,tb_rocket,kernel")
    // ).init();
}

fn get_static_path() -> PathBuf {
    let path = std::env::var("STATIC_WWW").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../frontend/public").to_string()
    });

    Path::new(&path)
        .canonicalize()
        .unwrap_or_else(|_| panic!("STATIC_WWW Path {} does not exist", path))
}

fn get_socket_address() -> SocketAddr {
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    addr.parse::<SocketAddr>()
        .unwrap_or_else(|_| panic!("BIND_ADDR '{}' could not be parsed", addr))
}
