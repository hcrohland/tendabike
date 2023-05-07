#![feature( decl_macro)]
#![feature(drain_filter)]
#![warn(clippy::all)]


#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

mod p_rocket;
mod drivers;
mod domain;

use domain::*;

use drivers::persistence::*;
use domain::presentation::Presentation;

fn main() {
    // setup environment. Includes Config and logging
    init_environment();

    p_rocket::Server::start();
}

pub fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("warn")
    ).init();
}
