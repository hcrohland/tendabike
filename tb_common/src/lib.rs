
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
use anyhow::Context;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

use chrono::{
    Utc,
    DateTime,
    TimeZone
};

pub fn parse_time (time: Option<String>) -> TbResult<Option<DateTime<Utc>>> {
    if let Some(time) = time {
        return Ok(Some(DateTime::parse_from_rfc3339(&time).context("could not parse time")?.with_timezone(&Utc)))
    }
    Ok(None)
}
