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
extern crate simplelog;

extern crate dotenv;

use self::diesel::prelude::*;

use simplelog::*;

use std::cmp::min;
use std::env;

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

use chrono::{DateTime, TimeZone, Utc};

type AppConn = diesel::PgConnection;

#[database("app_db")]
pub struct AppDbConn(AppConn);

embed_migrations!();

use rocket::Rocket;

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = AppDbConn::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*conn) {
        Ok(()) => Ok(rocket),
        Err(e) => {
            error!("Failed to run database migrations: {:?}", e);
            Err(rocket)
        }
    }
}

use rocket::fairing::AdHoc;

pub fn ignite_rocket() -> rocket::Rocket {
    dotenv::dotenv().ok();
    // Initialize server

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions::default()
        .to_cors()
        .expect("Could not set CORS options");

    let ship = rocket::ignite()
        // add database pool
        .attach(AppDbConn::fairing())
        // run database migrations
        .attach(AdHoc::on_attach("TendaBike Database Migrations", run_db_migrations))
        .attach(cors)
        // mount all the endpoints from the module
        .mount(
            "/",
            rocket_contrib::serve::StaticFiles::from(
                env::var("STATIC_WWW").unwrap_or_else(|_|
                    concat!(env!("CARGO_MANIFEST_DIR"),"/../tb_svelte/public").into()
                )
            )
        )
        .mount("/user", user::routes())
        .mount("/types", types::routes())
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
        .mount("/attach", attachment::routes());
    let config = ship.config().clone();
    let ship = ship.manage(config);
    tb_strava::attach_rocket(ship)
}

fn init_logging() {
    let config = simplelog::ConfigBuilder::new().set_time_format("%F %T".to_string()).build();
    if let Err(_) = TermLogger::init(
        LevelFilter::Info,
        config.clone(),
        simplelog::TerminalMode::Stdout) 
    {
        SimpleLogger::init(LevelFilter::Info, config).expect ("could not get logger");
    }
}

pub fn init_environment() {
    dotenv::dotenv().ok();

    init_logging();
}

#[derive(Debug)]
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

#[derive(Serialize, Debug, Default)]
pub struct PartAttach {
    parts: Vec<part::Part>,
    attachments: Vec<attachment::AttachmentDetail>
}
