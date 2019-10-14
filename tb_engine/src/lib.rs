#![feature(proc_macro_hygiene, decl_macro, result_map_or_else)]
#![warn(clippy::all)]

#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_derive_newtype;

#[macro_use] extern crate newtype_derive;
#[macro_use] extern crate log;
extern crate simplelog;
extern crate chrono;

extern crate dotenv;

extern crate tb_common;

use std::cmp::min;
use self::diesel::prelude::*;
use rocket_contrib::templates::Template;

use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};

use std::env;
use std::fs::File;

pub mod schema;
pub mod user;

pub mod types;
use types::*;

pub mod part;
use part::{PartId};

pub mod activity;
use activity::Activity;

pub mod attachment;

use tb_common::error::*;

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
    dotenv::dotenv().ok();
    // Initialize server
    rocket::ignite()
       // add config object
        .manage(Config::default())
        // add database pool
        .attach(AppDbConn::fairing())
        .attach(Template::fairing())


        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        .mount("/types", types::routes())
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
        .mount("/attach", attachment::routes())
}

fn init_logging (){
    const LOGFILE_NAME: & str = "tendabike.log";
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

pub fn init_environment () {
    dotenv::dotenv().ok();

    init_logging();       
}

pub struct Usage {
    // oldest activity
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
    /// number of activities
    pub count: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Factor {
    Add = 1,
    Sub = -1,
    No = 0,
}

impl Usage {
    pub fn none() -> Usage {
        Usage {
            start: Utc::now(),
            time: 0,
            climb: 0,
            descend: 0,
            power: 0,
            distance: 0,
            count: 0,
        }
    }

    /// Add an activity to of a usage
    /// 
    /// If the descend value is missing, assume descend = climb
    pub fn add_activity (self, act: &Activity, factor: Factor) -> Usage {

        let factor = factor as i32;
        Usage {
            start: min(self.start, act.start),
            time: self.time + act.time.unwrap_or(0) * factor,
            climb: self.climb + act.climb.unwrap_or(0) * factor,
            descend: self.descend + act.descend.unwrap_or_else(|| act.climb.unwrap_or(0)) * factor,
            power: self.power + act.power.unwrap_or(0) * factor,
            distance: self.distance + act.distance.unwrap_or(0) * factor,
            count: self.count + factor,
        }
    }
}
/* 
mod error {
    use std::fmt;
    use std::error;

    #[derive(Debug)]
    pub enum MyError {
        DbError (diesel::result::Error),
        NotAuth (String),
        NotFound (String),
        Forbidden (String),
        BadRequest (String),
        Conflict(String),
        AnyErr (String),
    }

    impl fmt::Display for MyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MyError::NotAuth(x)     => write!(f, "Not authorized {}", x),
                MyError::Forbidden(x)   => write!(f, "Forbiddenrequest: {}", x),
                MyError::NotFound(x)    => write!(f, "Could not find object: {}", x),
                MyError::BadRequest(x)  => write!(f, "Bad Request: {}", x),
                MyError::Conflict(x)  => write!(f, "Conflict: {}", x),
                MyError::AnyErr(x)      => write!(f, "Internal error detected: {}", x),
                MyError::DbError(ref e) => e.fmt(f),
            }
        }
    }

    impl error::Error for MyError {
        fn source(&self) -> Option<&(dyn error::Error + 'static)> {
            match *self {
                MyError::DbError(ref e) => Some(e),
                _ => None,
            }
        }
    }

    impl std::convert::From<diesel::result::Error> for MyError {
        fn from(err: diesel::result::Error) -> MyError {
            MyError::DbError(err)
        }
    }

    use rocket::http::Status;
    use rocket::response::{Response, Responder};
    use rocket::request::Request;
    use diesel::result::Error as DieselError;

    impl<'r> Responder<'r> for MyError {
        fn respond_to(self, _: &Request) -> Result<Response<'r>, Status> {
            use diesel::result::DatabaseErrorKind::*;
            warn!("{}", self);
            match self {
                MyError::DbError(error) => {
                    match error  {
                        DieselError::NotFound => Err(Status::NotFound),
                        DieselError::DatabaseError(ForeignKeyViolation,_) => Err(Status::BadRequest),
                        _ => Err(Status::InternalServerError)
                    }
                },
                MyError::NotFound(_) => Err(Status::NotFound),
                MyError::NotAuth(_)  => Err(Status::Unauthorized),
                MyError::Forbidden(_) => Err(Status::Forbidden),
                MyError::BadRequest(_) => Err(Status::BadRequest),
                MyError::Conflict(_) => Err(Status::Conflict),
                _ => Err(Status::InternalServerError),
            }
        }
    }
}
*/

use chrono::{
    Utc,
    Local,
    DateTime,
    TimeZone
};


fn parse_time (time: Option<String>) -> Option<DateTime<Utc>> {
    time.map(|time| Local.datetime_from_str(&time, "%FT%T").expect(&*format!("could not parse time {}", time)).with_timezone(&Utc))
}

type PartList = Vec<part::Part>;

