pub(crate) use anyhow::Context;

pub mod activity;
pub mod auth;
pub mod gear;
pub mod ui;
pub mod webhook;

pub use crate::*;

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

pub fn attach_rocket(ship: rocket::Rocket) -> rocket::Rocket {
    dotenv::dotenv().ok();
    let config = ship.config().clone();

    ship
        // add oauth2 flow
        .attach(auth::fairing(&config))
        .mount("/strava", ui::routes())
}
