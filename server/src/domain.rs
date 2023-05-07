pub mod types;
pub mod part;
pub mod activity;
pub mod attachment;
pub mod user;
pub mod error;

use std::collections::HashMap;

pub use part::{Part, PartId};
pub use activity::{Activity, ActivityId};
pub use attachment::{Attachment, AttachmentDetail};
pub use user::{Person, User};
pub use types::*;

use chrono::{DateTime, Utc, TimeZone};
use diesel::{self, QueryDsl, RunQueryDsl};
use diesel::prelude::*;

use crate::drivers::persistence::{schema, AppConn};
use super::{Error, TbResult, Context, Connection};

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
    pub fn new(activities: Vec<activity::Activity>, parts: Vec<Part>, attachments: Vec<AttachmentDetail>) -> Self {
            Summary {activities,parts,attachments}
    }

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

    pub fn first(&self) -> ActivityId {
        self.activities[0].id
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

