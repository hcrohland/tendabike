#![feature( decl_macro)]
#![warn(clippy::all)]

use std::path::{Path, PathBuf};

use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket_contrib::database;
use log::{info,warn};
use crate::error::*;

use kernel::{domain, s_diesel};
mod routes;
mod strava;
mod error;
mod jwt;


// AppDbConn needs to be published to be used in Rocket
// It should not be used outside presentation
#[database("app_db")]
pub struct AppDbConn(s_diesel::AppConn);

use rocket::{Outcome, request::{FromRequest, self}, Request, http::Status};
use domain::{Person, User, AnyResult};

mod user;
use user::*;

pub fn start() {
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
        .mount("/",rocket_contrib::serve::StaticFiles::from(get_static_path()))
        .mount("/user", routes::user::routes())
        .mount("/types", routes::types::routes())
        .mount("/part", routes::part::routes())
        .mount("/part", routes::attachment::routes())
        .mount("/activ", routes::activity::routes())
        .mount("/strava", strava::ui::routes());
        
    // add oauth2 flow
    let config = ship.config().clone();
    ship.attach(strava::fairing(&config))
        .attach(rocket::fairing::AdHoc::on_launch("Launch Message", |rocket| {
            let c = rocket.config();
            info!("\n\n TendaBike running on {}:{}\n\n", c.address, c.port);
        }))
        .launch();
}

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = AppDbConn::get_one(&rocket).expect("database connection");
    s_diesel::run_db_migrations(&conn);
    Ok(rocket)
}

fn get_static_path () -> PathBuf {
    let path = std::env::var("STATIC_WWW").unwrap_or_else(
        |_| concat!(env!("CARGO_MANIFEST_DIR"),"/../../../frontend/public").to_string()
    );
    
    Path::new(&path).canonicalize().expect(&format!("STATIC_WWW Path {} does not exist", path))
}