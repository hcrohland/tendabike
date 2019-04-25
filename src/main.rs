#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
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

    rocket::ignite()
        .manage(Config::default())
        .mount("/", routes![index]).launch();
}