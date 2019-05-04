
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket;
extern crate rocket_contrib;
extern crate diesel;
#[macro_use] extern crate log;
extern crate dotenv;

use rocket::State;
use rocket_contrib::json::Json;

extern crate tendabike;

use tendabike::Config;
use tendabike::db;
use tendabike::user::*;

#[get("/")]
fn index() -> &'static str {
    "Hello, want to tend your bikes?"
}

#[get("/config")]
fn index_conf(conf: State<Config>, user: User) -> String {
    format!( "{}, user id {}\n", conf.greeting, user.get_id())
}

#[get("/db")]
fn index_db(conn: tendabike::AppDbConn) -> String {
    db::get_greeting(&conn)
}

#[derive(Serialize)]
struct Greeting {
    greeting: String,
    user_id: i32,
}

#[get("/json")]
fn index_json(conn: tendabike::AppDbConn, user: User) -> Json<Greeting> {
    Json( Greeting {
        greeting: index_db(conn),
        user_id: user.get_id(),
    })
}
#[get("/exit")]
fn server_exit(admin: Admin) {
    info!( "user id {} requested shutdown\n", admin.get_id());
    std::process::exit(0);
}

fn main() {

    tendabike::init_environment();

    trace!("Trace");
    debug!("Debug");
    info!("Info");
    error!("Error");

    rocket::ignite()
        .manage(Config::default())
        .attach(tendabike::AppDbConn::fairing())
        .mount("/", routes![index, index_conf, index_db, index_json, server_exit])
        .launch();
}