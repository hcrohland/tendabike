#![feature(proc_macro_hygiene, decl_macro)]
#![feature(drain_filter)]
#![warn(clippy::all)]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate rocket_cors;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_newtype;
#[macro_use]
extern crate diesel_migrations;

#[macro_use]
extern crate newtype_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;
extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;

extern crate dotenv;

#[macro_use]
extern crate thiserror;

extern crate reqwest;
extern crate jsonwebtoken;

use self::diesel::prelude::*;

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
