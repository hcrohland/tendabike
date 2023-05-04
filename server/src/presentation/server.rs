use rocket::Rocket;
use rocket::fairing::AdHoc;
use std::env;
use crate::*;


pub fn start () {
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
        // .mount("/user", user::routes())
        // .mount("/types", types::routes())
        // .mount("/part", part::routes())
        // .mount("/part", attachment::routes())
        // .mount("/activ", activity::routes())
        // .mount("/strava", drivers::strava::ui::routes())
        ;
        
        // add oauth2 flow
        let config = ship.config().clone();
        ship.attach(drivers::strava::auth::fairing(&config))
            .attach(rocket::fairing::AdHoc::on_launch("Launch Message", |rocket| {
                let c = rocket.config();
                eprintln!("\nInfo: TendaBike running on {}:{}\n", c.address, c.port);
            }))
            .launch();
}

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = AppDbConn::get_one(&rocket).expect("database connection");
    crate::run_db_migrations(&conn);
    Ok(rocket)
}
