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
extern crate newtype_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;
extern crate chrono;
extern crate simplelog;

extern crate dotenv;

use self::diesel::prelude::*;
use rocket_contrib::templates::Template;

use simplelog::{CombinedLogger, LevelFilter, TermLogger, WriteLogger};

use std::cmp::min;
use std::env;
use std::fs::File;

pub mod schema;
pub mod user;

pub mod types;
use types::*;

pub mod part;
use part::PartId;

pub mod activity;
use activity::Activity;

pub mod attachment;

extern crate tb_common;
extern crate tb_strava;
pub use tb_common::error::*;
pub use tb_common::*;

use anyhow::Context;

use chrono::{DateTime, Local, TimeZone, Utc};

type AppConn = diesel::PgConnection;

#[database("app_db")]
pub struct AppDbConn(AppConn);

pub struct Config {
    pub greeting: String,
}

impl Default for Config {
    fn default() -> Config {
        let greet = match env::var("TENDER_GREETING") {
            Ok(val) => val,
            Err(_e) => String::from("Hello, want to tend your bikes?"),
        };
        Config { greeting: greet }
    }
}

pub fn ignite_rocket() -> rocket::Rocket {
    use rocket_cors::*;
    dotenv::dotenv().ok();
    // Initialize server

    let allowed_origins =
        AllowedOrigins::some_exact(&[
            "http://localhost:5000", 
            "http://127.0.0.1:5000", 
            "http://localhost:8000", 
            "http://127.0.0.1:8000"]);

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        // allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        // allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("Could not set CORS options");

    let ship = rocket::ignite()
        // add config object
        .manage(Config::default())
        // add database pool
        .attach(AppDbConn::fairing())
        .attach(Template::fairing())
        .attach(cors)
        // mount all the endpoints from the module
        .mount(
            "/",
            rocket_contrib::serve::StaticFiles::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../tb_svelte/public"
            )),
        )
        .mount("/user", user::routes())
        .mount("/types", types::routes())
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
        .mount("/attach", attachment::routes());
    tb_strava::attach_rocket(ship)
}

fn init_logging() {
    const LOGFILE_NAME: &str = "tendabike.log";
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            simplelog::Config::default(),
            simplelog::TerminalMode::Stdout,
        )
        .expect("Couldn't get terminal logger"),
        WriteLogger::new(
            LevelFilter::Debug,
            simplelog::Config::default(),
            File::create(LOGFILE_NAME).expect("Couldn't create logfile"),
        ),
    ])
    .expect("Can't get logger.");
}

pub fn init_environment() {
    dotenv::dotenv().ok();

    init_logging();
}

pub struct Usage {
    // oldest activity
    pub start: DateTime<Utc>,
    // usage time
    pub time: i32,
    /// Usage distance
    pub distance: i32,
    /// Overall climbing
    pub climb: i32,
    /// Overall descending
    pub descend: i32,
    /// Overall descending
    pub power: i32,
    /// number of activities
    pub count: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Factor {
    Add = 1,
    Sub = -1,
    No = 0,
}

impl Usage {
    pub fn none(time: DateTime<Utc>) -> Usage {
        Usage {
            start: time,
            time: 0,
            climb: 0,
            descend: 0,
            power: 0,
            distance: 0,
            count: 0,
        }
    }

    /// Add an activity to of a usage
    ///
    /// If the descend value is missing, assume descend = climb
    pub fn add_activity(self, act: &Activity, factor: Factor) -> Usage {
        let factor = factor as i32;
        Usage {
            start: min(self.start, act.start),
            time: self.time + act.time.unwrap_or(0) * factor,
            climb: self.climb + act.climb.unwrap_or(0) * factor,
            descend: self.descend + act.descend.unwrap_or_else(|| act.climb.unwrap_or(0)) * factor,
            power: self.power + act.power.unwrap_or(0) * factor,
            distance: self.distance + act.distance.unwrap_or(0) * factor,
            count: self.count + factor,
        }
    }
}

type PartList = Vec<part::Part>;
