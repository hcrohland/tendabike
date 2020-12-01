#![warn(clippy::all)]

use crate::error::*;

pub(crate) use anyhow::Context;

pub mod activity;
pub mod auth;
pub mod gear;
pub mod schema;
pub mod ui;
pub mod webhook;

const TB_URL: &str = "http://localhost";

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("authorization needed: {0}")]
    Authorize(&'static str),
}

use serde_json::Value as jValue;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct JSummary {
    activities: Vec<jValue>,
    parts: Vec<jValue>,
    attachments: Vec<jValue>
}

type AppConn = diesel::PgConnection;

#[database("auth_db")]
pub struct AppDbConn(AppConn);

embed_migrations!("migrations");

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

pub fn attach_rocket(ship: rocket::Rocket) -> rocket::Rocket {
    dotenv::dotenv().ok();
    let config = ship.config().clone();

    ship
        // add database pool
        .attach(AppDbConn::fairing())
        // run database migrations
        .attach(AdHoc::on_attach("Strava Database Migrations", run_db_migrations))
        // add oauth2 flow
        .attach(auth::fairing(&config))
        .mount("/strava", ui::routes())
}
