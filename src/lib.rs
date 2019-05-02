

#[macro_use] extern crate log;
extern crate simplelog;


use diesel::prelude::*;
use diesel::pg::PgConnection;


use std::env;

use simplelog::{
    CombinedLogger,
    LevelFilter,
    TermLogger,
    WriteLogger,
};
use std::fs::File;

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

pub fn init_logging (){
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

    trace!("Trace");
    debug!("Debug");
    info!("Info");
    error!("Error");
}