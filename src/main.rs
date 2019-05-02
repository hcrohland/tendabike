#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

#[macro_use] extern crate diesel;

extern crate dotenv;

extern crate tendabike;

use rocket::State;

#[database("app_db")]
struct LogsDbConn(diesel::PgConnection);

use tendabike::Config;
pub mod db;
pub mod schema;




#[get("/")]
fn index(conf: State<Config>) -> String {
    conf.greeting.clone()
}

#[get("/db")]
fn index_db(conn: LogsDbConn) -> String {
    db::get_greeting(&conn)
}

fn main() {
// read environment variables from file
    dotenv::dotenv().ok();

    tendabike::init_logging();

    rocket::ignite()
        .manage(Config::default())
        .mount("/", routes![index, index_db])
        .attach(LogsDbConn::fairing())
        .launch();
}