
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

//#[macro_use] 
extern crate diesel;

#[macro_use] extern crate log;

extern crate dotenv;

extern crate tendabike;

use rocket::State;

#[database("app_db")]
struct AppDbConn(diesel::PgConnection);

use self::tendabike::Config;
use self::tendabike::db;


#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

#[get("/config")]
fn index_conf(conf: State<Config>) -> String {
    conf.greeting.clone()
}

#[get("/db")]
fn index_db(conn: AppDbConn) -> String {
    db::get_greeting(&conn)
}

fn main() {
// read environment variables from file
    dotenv::dotenv().ok();

    tendabike::init_logging();

    trace!("Trace");
    debug!("Debug");
    info!("Info");
    error!("Error");

    rocket::ignite()
        .manage(Config::default())
        .mount("/", routes![index, index_conf, index_db])
        .attach(AppDbConn::fairing())
        .launch();
}