#![feature(proc_macro_hygiene, decl_macro)]
#![warn(clippy::all)]

#[macro_use] extern crate log;

#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
extern crate reqwest;
#[macro_use] extern crate lazy_static;
extern crate time;

pub mod user;

pub struct Config {
}

impl Default for Config {
    fn default() -> Config {
        // use std::env;

        Config {
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
            Mess(message: &'static str) {
                description("Any error")
                display("Error: {}", message)
            }
            Parse {
                from(std::num::ParseIntError)   
                from(serde_json::error::Error)
            }
            Request(err: reqwest::Error) {
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
pub type MyResult<T> = Result<T, Error>;