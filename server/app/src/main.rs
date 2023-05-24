#![warn(clippy::all)]

use kernel::s_diesel::DbPool;
use std::{path::{Path, PathBuf}};

fn main() -> anyhow::Result<()> {
    // setup environment. Includes Config and logging
    init_environment();
    
    let db = DbPool::init()?;
    let path = get_static_path();

    #[cfg(axum)]
    tb_axum::start(db, path);
    #[cfg(not(axum))]
    tb_rocket::start(DbPool(db), path);

    Ok (())
}

fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    // env_logger::Builder::from_env(
    //     env_logger::Env::default().default_filter_or("tendabike,tb_rocket,kernel")
    // ).init();
}

fn get_static_path () -> PathBuf {
    let path = std::env::var("STATIC_WWW").unwrap_or_else(
        |_| concat!(env!("CARGO_MANIFEST_DIR"),"/../../frontend/public").to_string()
    );
    
    dbg!(Path::new(&path).canonicalize().unwrap_or_else(|_| panic!("STATIC_WWW Path {} does not exist", path)))
}
