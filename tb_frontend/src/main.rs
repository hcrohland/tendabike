#![feature(proc_macro_hygiene, decl_macro)]
#![warn(clippy::all)]

extern crate simplelog;
// #[macro_use] extern crate log;

#[macro_use] 
extern crate rocket;
// #[macro_use] 
extern crate rocket_contrib;
extern crate reqwest;
// #[macro_use] extern crate lazy_static;
extern crate time;
extern crate anyhow;
pub use anyhow::Context;

extern crate tb_common;
use tb_common::*;

pub mod user;
pub use user::*;

mod dashboard;


pub struct Config {
}

impl Default for Config {
    fn default() -> Config {
        // use std::env;

        Config {
        }
    }
}


use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};
use rocket_contrib::templates::Template;

fn init_logging (){
    use std::fs::File;
    const LOGFILE_NAME: & str = "tb_frontend.log";
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, simplelog::Config::default(), simplelog::TerminalMode::Stdout).expect("Couldn't get terminal logger"),
        WriteLogger::new(
            LevelFilter::Debug,
            simplelog::Config::default(),
            File::create(LOGFILE_NAME).expect("Couldn't create logfile"),
        ),
    ])
    .expect("Can't get logger.");

}

fn init_environment () {
    dotenv::dotenv().expect("Couldn't read environment");

    init_logging();       
}

pub fn ignite_rocket () -> rocket::Rocket {
    dotenv::dotenv().ok();
    // Initialize server
    rocket::ignite()
        // add config object
        .manage(Config::default())
        // add Template support
        .attach(Template::fairing())
        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        .mount("/user", user::routes())
        .mount("/dashboard", dashboard::routes())
}

fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket().launch();
}

