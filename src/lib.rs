#![feature(proc_macro_hygiene, decl_macro, result_map_or_else)]

#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

#[macro_use] extern crate diesel;

#[macro_use] 
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

//use std::error;
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

impl Default for Config {
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

pub fn ignite_rocket () -> rocket::Rocket {
    // Initialize server
    rocket::ignite()
       // add config object
        .manage(Config::default())
        // add database pool
        .attach(AppDbConn::fairing())

        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
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
    pub op: Option<for<'r> fn(&'r mut i32, i32)>,
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

impl Usage {
    pub fn none() -> Usage {
        Usage {
            op: None,
            start: Utc::now(),
            time: 0,
            climb: 0,
            descend: 0,
            power: 0,
            distance: 0,
        }
    }
}

// enum Error {
//     DbError(diesel::result::Error),
//     NotAuth (String),
//     NotFound (String),
//     AnyErr,
// }


use rocket::request::Request;
use rocket::response::{self, Responder};

#[derive(Debug)]
struct DbResult<T> (diesel::result::QueryResult<T>);

/// If `self` is Err(NotFound), respond with None to generate a 404 response
/// otherwise responds with the wrapped `Responder`.  
impl<'r, R: Responder<'r>> Responder<'r> for DbResult<R> 
    where R: std::fmt::Debug
{
    fn respond_to(self, req: &Request) -> response::Result<'r> {
        let res = match self.0 {
            Err(diesel::result::Error::NotFound) => None,
            _ => Some(self.0),
        };
        res.respond_to(req)
    }
}

