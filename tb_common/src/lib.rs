
extern crate anyhow;
#[macro_use] 
extern crate thiserror;

extern crate chrono;
extern crate rocket;
#[macro_use] 
extern crate rocket_contrib;
#[macro_use] 
extern crate log;
extern crate diesel;

pub mod error;
pub use error::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use chrono::{
    Utc,
    Local,
    DateTime,
    TimeZone
};


pub fn parse_time (time: Option<String>) -> Option<DateTime<Utc>> {
    time.map(|time| Local.datetime_from_str(&time, "%FT%T").expect(&*format!("could not parse time {}", time)).with_timezone(&Utc))
}


