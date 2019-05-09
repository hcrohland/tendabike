#![feature(proc_macro_hygiene, decl_macro, never_type)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

#[macro_use] extern crate diesel;

//#[macro_use] 
extern crate log;
extern crate simplelog;
extern crate chrono;

extern crate dotenv;

use chrono::{
    Utc,
    DateTime,
};

use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};
use std::env;
use std::fs::File;

//pub mod db;
pub mod schema;
pub mod user;
//pub mod greetings;
pub mod part;
pub mod activity;

type AppConn = diesel::PgConnection;

#[database("app_db")]
pub struct AppDbConn(AppConn);

pub struct Config {
    pub greeting: String,
}

impl Default for Config{
    fn default() -> Config {
        let greet = match env::var("TENDER_GREETING") {
            Ok(val) => val,
            Err(_e) => String::from("Hello, want to tend your bikes?"),
        };
        Config {
            greeting: greet,
        }
    }
}

fn init_logging (){
    const LOGFILE_NAME: &'static str = "tendabike.log";
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, simplelog::Config::default()).expect("Couldn't get terminal logger"),
        WriteLogger::new(
            LevelFilter::Debug,
            simplelog::Config::default(),
            File::create(LOGFILE_NAME).expect("Couldn't create logfile"),
        ),
    ])
    .expect("Can't get logger.");

}

pub fn init_environment () -> () {
    dotenv::dotenv().ok();

    init_logging();       
}

pub struct Usage {
    // start time
    pub start: DateTime<Utc>,
    // usage time
    pub time: i32,
    /// Usage distance
	pub distance: i32,
	/// Overall climbing
    pub climb: i32,
    /// Overall descending
	pub descend: i32,
    /// Overall descending
	pub power: i32,
}