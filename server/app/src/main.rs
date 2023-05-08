#![warn(clippy::all)]

fn main() {
    // setup environment. Includes Config and logging
    init_environment();

    tb_rocket::start();
}

pub fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("tendabike,tb_rocket,kernel")
    ).init();
}
