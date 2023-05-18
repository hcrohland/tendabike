#![warn(clippy::all)]

use kernel::*;

fn main() -> anyhow::Result<()> {
    // setup environment. Includes Config and logging
    init_environment();

    let db = s_diesel::init_connection_pool()?;
    Ok (tb_rocket::start(db))
}

fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("tendabike,tb_rocket,kernel")
    ).init();
}
