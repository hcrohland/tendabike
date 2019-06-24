#![feature(proc_macro_hygiene, decl_macro, result_map_or_else)]

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

pub mod schema;
pub mod user;

pub mod part;
use part::PartId as PartId;

pub mod activity;
use activity::Activity as Activity;

use error::MyError as MyError;

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

        // mount all the endpoints from the module
        .mount("/", rocket_contrib::serve::StaticFiles::from(concat!(env!("CARGO_MANIFEST_DIR"), "/www")))
        .mount("/part", part::routes())
        .mount("/activ", activity::routes())
}

fn init_logging (){
    const LOGFILE_NAME: & str = "tendabike.log";
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

pub fn init_environment () {
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
    /// number of activities
    pub count: i32,
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
    // pub fn add (&mut self, rh: &Usage) {
    //     self.time += rh.time;
    //     self.climb += rh.climb;
    //     self.descend += rh.descend;
    //     self.power += rh.power;
    //     self.distance += rh.distance;
    //     self.count += rh.distance;
    // }
}

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
        AnyErr (String),
    }

    impl fmt::Display for MyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                MyError::NotAuth(x)     => write!(f, "Not authorized {}", x),
                MyError::Forbidden(x)   => write!(f, "Forbiddenrequest: {}", x),
                MyError::NotFound(x)    => write!(f, "Could not find object: {}", x),
                MyError::BadRequest(x)  => write!(f, "Bad Request: {}", x),
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
                _ => Err(Status::InternalServerError),
            }
        }
    }
}

pub type TbResult<T> = Result<T, error::MyError>;

