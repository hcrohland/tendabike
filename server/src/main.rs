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

use std::cmp::{max,min};
use std::env;

pub mod jwt;
pub mod error;
pub mod strava;
pub mod schema;
pub mod user;

pub mod types;
use types::*;

pub mod part;
use part::PartId;

pub mod activity;
use activity::Activity;

pub mod attachment;

pub use error::*;

use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};

use rocket::Rocket;
use rocket::fairing::AdHoc;

pub type AppConn = diesel::PgConnection;

fn main() {
    // setup environment. Includes Config and logging
    init_environment();

    // start the server
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
                    concat!(env!("CARGO_MANIFEST_DIR"),"/../frontend/public").into()
                )
            )
        )
        .mount("/user", user::routes())
        .mount("/types", types::routes())
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
        .mount("/attach", attachment::routes())
        .mount("/strava", strava::ui::routes());
        
        // add oauth2 flow
        let config = ship.config().clone();
        ship.attach(strava::auth::fairing(&config))
            .attach(rocket::fairing::AdHoc::on_launch("Launch Message", |rocket| {
                let c = rocket.config();
                eprintln!("\nInfo: TendaBike running on {}:{}\n", c.address, c.port);
            }))
            .launch();
}
#[database("app_db")]
pub struct AppDbConn(AppConn);

embed_migrations!();

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

pub fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("warn")
    ).init();
}

pub fn parse_time (time: Option<String>) -> TbResult<Option<DateTime<Utc>>> {
    if let Some(time) = time {
        return Ok(Some(DateTime::parse_from_rfc3339(&time).context("could not parse time")?.with_timezone(&Utc)))
    }
    Ok(None)
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
pub struct Summary {
    activities: Vec<activity::Activity>,
    parts: Vec<part::Part>,
    attachments: Vec<attachment::AttachmentDetail>
}

impl Summary {
    pub fn append(&mut self, new: &mut Self) {
        self.activities.append(&mut new.activities);
        self.parts.append(&mut new.parts);
        self.attachments.append(&mut new.attachments);
    }
}
