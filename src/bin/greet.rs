
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

use tendabike::Config;
use tendabike::db;
use tendabike::user;
use user::Person;

#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

#[get("/config")]
fn index_conf(conf: State<Config>, user: user::User) -> String {
    format!( "{}, user id {}\n", conf.greeting, user.get_id())
}

#[get("/db")]
fn index_db(conn: AppDbConn) -> String {
    db::get_greeting(&conn)
}

#[get("/exit")]
fn server_exit(admin: user::Admin) {
    info!( "user id {} requested shutdown\n", admin.get_id());
    std::process::exit(0);
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
        .attach(AppDbConn::fairing())
        .mount("/", routes![index, index_conf, index_db, server_exit])
        .launch();
}