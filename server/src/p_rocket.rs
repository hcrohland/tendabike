use rocket::Rocket;
use rocket::fairing::AdHoc;
use rocket_contrib::database;
use log::{info,trace,warn,error};
use serde_derive::*;
use crate::*;
use crate::error::*;

mod routes;
pub(super) mod strava;
mod error;
mod jwt;
use error::*;


// AppDbConn needs to be published to be used in Rocket
// It should not be used outside presentation
#[database("app_db")]
pub struct AppDbConn(AppConn);

use rocket::{Outcome, request::{FromRequest, self}, Request, http::Status};
use crate::domain::{user::User, error::TbResult};
use p_rocket::Person;

struct RUser<'a> ( &'a User );

fn readuser (request: &Request) -> TbResult<User> {
    let id = strava::get_id(request)?;
    let conn = request.guard::<AppDbConn>().expect("No db request guard").0;
    User::read(id, &conn)
}

impl<'a, 'r> FromRequest<'a, 'r> for RUser<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> rocket::Outcome<RUser<'a>, (rocket::http::Status, &'a anyhow::Error), ()> {
        let user_result = request.local_cache(|| readuser(request));

        match user_result.as_ref() {
            Ok(x) => Outcome::Success(RUser(x)),
            Err(e) => Outcome::Failure((Status::Unauthorized, e)),
        }
    }
}

pub struct Admin<'a> (&'a User);

impl<'a, 'r> FromRequest<'a, 'r> for Admin<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Admin<'a>, &'a anyhow::Error> {
        let RUser(user) = request.guard::<RUser>()?;

        if user.is_admin() {
            Outcome::Success(Admin(user))
        } else {
            Outcome::Forward(())
        }
    }
}

impl Person for Admin<'_> {
    fn get_id(&self) -> i32 {
        self.0.get_id()
    }
    fn is_admin(&self) -> bool {
        assert!(self.0.is_admin());
        true
    }
}

#[derive(Debug)]
pub struct Server {}

impl domain::presentation::Presentation for Server {
    fn start() -> Self {
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
                    std::env::var("STATIC_WWW").unwrap_or_else(|_|
                        concat!(env!("CARGO_MANIFEST_DIR"),"/../frontend/public").into()
                    )
                )
            )
            .mount("/user", routes::user::routes())
            .mount("/types", routes::types::routes())
            .mount("/part", routes::part::routes())
            .mount("/part", routes::attachment::routes())
            .mount("/activ", routes::activity::routes())
            .mount("/strava", strava::ui::routes())
            ;
            
            // add oauth2 flow
            let config = ship.config().clone();
            ship.attach(strava::fairing(&config))
                .attach(rocket::fairing::AdHoc::on_launch("Launch Message", |rocket| {
                    let c = rocket.config();
                    info!("\n\n TendaBike running on {}:{}\n\n", c.address, c.port);
                }))
                .launch();
            Server {}
    }
}

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = AppDbConn::get_one(&rocket).expect("database connection");
    drivers::persistence::run_db_migrations(&conn);
    Ok(rocket)
}