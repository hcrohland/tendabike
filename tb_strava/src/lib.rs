#![feature(proc_macro_hygiene, decl_macro)]
#![warn(clippy::all)]

#[macro_use] extern crate log;

#[macro_use] extern crate diesel;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;
extern crate reqwest;
extern crate rocket_oauth2;
#[macro_use] extern crate lazy_static;
extern crate time;
#[macro_use] 
extern crate anyhow;
#[macro_use] 
extern crate thiserror;
extern crate tb_common;
use tb_common::*;

pub(crate) use anyhow::Context;
use std::env;

pub mod auth;
pub mod ui;
pub mod schema;
pub mod activity;
pub mod gear;

const TB_URI: &str = "http://localhost:8000";

type AppConn = diesel::PgConnection;

#[database("strava_db")]
pub struct StravaDbConn(AppConn);

pub struct Config {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Error, Debug)]
pub enum StravaError {
    #[error("authorization needed: {0}")]
    Authorize(&'static str),
}

pub fn attach_rocket (ship: rocket::Rocket) -> rocket::Rocket {
    dotenv::from_filename(".secrets").expect("Couldn't read secrets");
    ship
        // add config object
        .manage(Config {
            client_id:      env::var("CLIENT_ID").expect("Couldn't read var CLIENT_ID"),
            client_secret:  env::var("CLIENT_SECRET").expect("Couldn't read var CLIENT_SECRET"),
        })
        // add database pool
        .attach(StravaDbConn::fairing())
        // add oauth2 flow
        .attach(auth::fairing())
        .mount("/strava", ui::routes())
}