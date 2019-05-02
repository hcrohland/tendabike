#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate simplelog;

extern crate dotenv;

extern crate tendabike;

use rocket::State;
use tendabike::Config;


#[get("/")]
fn index(conf: State<Config>) -> String {
    conf.greeting.clone()
}

fn main() {
// read environment variables from file
    dotenv::dotenv().ok();

    tendabike::init_logging();

    rocket::ignite()
        .manage(Config::default())
        .mount("/", routes![index]).launch();
}