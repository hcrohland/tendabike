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

pub mod auth;
pub mod user;
pub mod schema;
pub mod tb;
pub mod activity;
pub mod gear;

const TB_URI: &str = "http://localhost:8000";

type AppConn = diesel::PgConnection;

#[database("strava_db")]
pub struct AppDbConn(AppConn);

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

#[macro_use] extern crate quick_error;

mod error {
    use rocket::request::Request;
    use rocket::response::{Response, Responder};
    use std::io::Cursor;
    use rocket::http::{Status, ContentType};

    quick_error! {
        #[derive(Debug)]
        pub enum Error {
            Authorize (r: &'static str) {
                description("You need to authorize")
                display("No authorization due to {}", r)
            }
            Mess(message: String) {
                description("Any error")
                display("Error: {}", message)
            }
            Json(string: String, err: serde_json::error::Error) {
                display("Json error: {} in '{}'", err, string)
                description(err.description())
                cause(err)
                from(err: serde_json::error::Error) -> ("unknown context".to_string(), err)
                context(r: &'a str, err: serde_json::error::Error) -> (r.to_string(), err)
                context(r: String, err: serde_json::error::Error) -> (r, err)
            }
            Parse {
                from(std::num::ParseIntError)   
            }
            OAuth(err: rocket_oauth2::hyper_sync_rustls_adapter::Error) {
                from()
            }
            Database(err: diesel::result::Error) {
                display("Database error: {}", err)
                from()
                cause(err)
            }
            Request(err: reqwest::Error) {
                display("Request error: {}", err)
                cause(err)
                description(err.description())
                from()
            }
        //     Other(err: Box<dyn std::error::Error>) {
        //         from()
        //         cause(&**err)
        //         description(err.description())
        //     }
        }
    }

    // Implement `Responder` for `error_chain`'s `Error` type
    // that we just generated
    impl<'r> Responder<'r> for Error {
        fn respond_to(self, _: &Request) -> ::std::result::Result<Response<'r>, Status> {
            // Render the whole error chain to a single string
            let rslt = format!("Error: {}", self);
            // let rslt = self.iter().skip(1).fold(start, |acc, ce| acc + &format!(", caused by: {}", ce));

            // Create JSON response
            let resp = json!({
                "status": "failure",
                "message": rslt,
            }).to_string();

            // Respond. The `Ok` here is a bit of a misnomer. It means we
            // successfully created an error response
            Ok(Response::build()
                .status(Status::BadRequest)
                .header(ContentType::JSON)
                .sized_body(Cursor::new(resp))
                .finalize())
        }
    }
}

 
pub use error::{Error};
pub use quick_error::ResultExt;

pub type MyResult<T> = Result<T, Error>;