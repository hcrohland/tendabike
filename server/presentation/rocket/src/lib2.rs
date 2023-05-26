#![feature( decl_macro)]
#![warn(clippy::all)]

use std::{path::{PathBuf}, ops::{Deref, DerefMut}};

use log::{info,warn};
use rocket::{Outcome, request::{FromRequest, self}, Request, http::Status, State};
use domain::{Person, User, AnyResult};


use crate::error::*;

use kernel::{domain, s_diesel::{self, DbPool}};
mod routes;
mod strava;
mod error;
mod jwt;

struct AppDbConn(s_diesel::Store);

impl<'a,'r> FromRequest<'a,'r> for AppDbConn {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let pool = request.guard::<State<DbPool>>();
        pool.map(|p| {
            AppDbConn(s_diesel::Store::new(&p).expect ("failed to get database connection"))
        })
    }
}

impl Deref for AppDbConn {
    type Target = s_diesel::Store;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AppDbConn {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

mod user;
use user::*;

pub fn start(db: DbPool, path: PathBuf) {
    let cors = rocket_cors::CorsOptions::default()
        .to_cors()
        .expect("Could not set CORS options");

    let ship = rocket::ignite()
        // add database pool
        .manage(db)
        .attach(cors)
        // mount all the endpoints from the module
        .mount("/",rocket_contrib::serve::StaticFiles::from(path))
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


