#![feature(proc_macro_hygiene, decl_macro)]
#![feature(drain_filter)]
#![warn(clippy::all)]

#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate rocket_cors;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_newtype;
#[macro_use]
extern crate diesel_migrations;

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

extern crate reqwest;
extern crate jsonwebtoken;

use self::diesel::prelude::*;

use std::cmp::{max,min};
use std::collections::HashMap;

mod schema;

mod presentation;
mod drivers;
mod domain;
use domain::*;
use types::*;
use part::{Part, PartId};
use activity::{Activity, ActivityId};
use attachment::{Attachment, AttachmentDetail};
use user::{Person, User};
use error::*;
use presentation::*;

use chrono::{DateTime, TimeZone, Utc};

pub type AppConn = diesel::PgConnection;

fn main() {
    // setup environment. Includes Config and logging
    init_environment();

    presentation::start()
}

embed_migrations!();

fn run_db_migrations (conn: &AppConn) {
    use schema::attachments::dsl::*;

    diesel::update(attachments)
        .filter(detached.is_null())
        .set(detached.eq(DateTime::<Utc>::MAX_UTC))
        .execute(conn)
        .expect("rewrite detached failed");

    embedded_migrations::run(conn)
        .expect("Failed to run database migrations: {:?}");
}

pub fn init_environment() {
    dotenv::dotenv().ok();

    // Default log level is warn
    env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("warn")
    ).init();
}

pub fn parse_time (time: Option<String>) -> TbResult<Option<DateTime<Utc>>> {
    if let Some(time) = time {
        return Ok(Some(DateTime::parse_from_rfc3339(&time).context("could not parse time")?.with_timezone(&Utc)))
    }
    Ok(None)
}

#[derive(Debug)]
pub struct Usage {
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
    pub fn add_activity(self, act: &Activity, factor: Factor) -> Usage {
        let factor = factor as i32;
        Usage {
            time: self.time + act.time.unwrap_or(0) * factor,
            climb: self.climb + act.climb.unwrap_or(0) * factor,
            descend: self.descend + act.descend.unwrap_or_else(|| act.climb.unwrap_or(0)) * factor,
            power: self.power + act.power.unwrap_or(0) * factor,
            distance: self.distance + act.distance.unwrap_or(0) * factor,
            count: self.count + factor,
        }
    }
}

#[derive(Serialize, Debug, Default)]
pub struct Summary {
    activities: Vec<activity::Activity>,
    parts: Vec<Part>,
    attachments: Vec<AttachmentDetail>
}

impl Summary {
    pub fn append(&mut self, new: &mut Self) {
        self.activities.append(&mut new.activities);
        self.parts.append(&mut new.parts);
        self.attachments.append(&mut new.attachments);
    }

    pub fn merge(self, new: Summary) -> Summary {
        let mut hash = SumHash::new(self);
        hash.merge(new);
        hash.collect()
    }
}

#[derive(Debug, Default)]
pub struct SumHash {
    activities: HashMap<ActivityId, Activity>,
    parts: HashMap<PartId, Part>,
    atts: HashMap<String, AttachmentDetail>,
}

impl SumHash {
    pub fn new(sum: Summary) -> Self {
        let mut hash = SumHash::default();
        hash.merge(sum);
        hash
    }

    pub fn merge(&mut self, ps: Summary)  {
        for act in ps.activities {
            self.activities.insert(act.id, act);
        }
        for part in ps.parts {
            self.parts.insert(part.id, part);
        }
        for att in ps.attachments {
            self.atts.insert(att.idx(), att);
        }
    }

    pub fn collect(self) -> Summary {
        Summary {
            activities: self.activities.into_iter().map(|(_,v)| v).collect(),
            parts: self.parts.into_iter().map(|(_,v)| v).collect(),
            attachments: self.atts.into_iter().map(|(_,v)| v).collect(),
        }
    }
}