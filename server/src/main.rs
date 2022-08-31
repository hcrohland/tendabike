#![feature(drain_filter)]
#![warn(clippy::all)]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[macro_use]
extern crate newtype_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate anyhow;
extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;

extern crate dotenv;

#[macro_use]
extern crate thiserror;

mod domain;

fn main () {
    unimplemented!()
}