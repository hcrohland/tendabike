#![feature(proc_macro_hygiene, decl_macro)]
#![warn(clippy::all)]

#[macro_use] extern crate log;

#[macro_use] extern crate diesel;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;
extern crate reqwest;
extern crate rocket_oauth2;
#[macro_use] extern crate lazy_static;
extern crate time;
#[macro_use] 
extern crate anyhow;
#[macro_use] 
extern crate thiserror;
extern crate tb_common;
use tb_common::*;

pub(crate) use anyhow::Context;

pub mod auth;
pub mod user;
pub mod schema;
pub mod tb;
pub mod activity;
pub mod gear;

const TB_URI: &str = "http://localhost:8000";

type AppConn = diesel::PgConnection;

#[database("strava_db")]
pub struct StravaDbConn(AppConn);

pub struct Config {
    pub client_id: String,
    pub client_secret: String,
}

impl Default for Config {
    fn default() -> Config {
        use std::env;

        Config {
            client_id:      env::var("CLIENT_ID").expect("Couldn't read var CLIENT_ID"),
            client_secret:  env::var("CLIENT_SECRET").expect("Couldn't read var CLIENT_SECRET"),
        }
    }
}

#[derive(Error, Debug)]
pub enum StravaError {
    #[error("authorization needed: {0}")]
    Authorize(&'static str),
}

extern crate simplelog;

use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};
use rocket_contrib::templates::Template;

fn init_logging (){
    use std::fs::File;
    const LOGFILE_NAME: & str = "tb_strava.log";
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
    dotenv::from_filename(".secrets").expect("Couldn't read secrets");

    init_logging();       
}

pub fn ignite_rocket () -> rocket::Rocket {
    dotenv::dotenv().ok();
    // Initialize server
    rocket::ignite()
        // add config object
        .manage(Config::default())
        // add database pool
        .attach(StravaDbConn::fairing())
        // add oauth2 flow
        .attach(auth::fairing())
        // add Template support
        .attach(Template::fairing())
        // redirects catcher
        .register(auth::catchers())
        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        // .mount("/auth", auth::routes())
        .mount("/", user::routes())
        .mount("/", tb::routes())
        // .mount("/activ", activity::routes())
        // .mount("/attach", attachment::routes())
}

fn main() {

    // setup environment. Includes Config and logging
    init_environment();

    // start the server
    ignite_rocket().launch();
}