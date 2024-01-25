//! This module contains the implementation of the `Summary` struct and its associated functions.
//!
//! The `Summary` struct is used to represent a summary of activities, parts, and attachments.
//! It contains three fields: `activities`, `parts`, and `attachments`, which are vectors of `Activity`,
//! `Part`, and `AttachmentDetail` structs, respectively.
//!
//! The `Summary` struct also has several associated functions, including `new`, `append`, `merge`, and `first`.
//!
//! Additionally, this module contains the `SumHash` struct and its associated functions, which are used
//! to efficiently merge multiple `Summary` structs together.

use serde_derive::Serialize;
use std::collections::HashMap;

use super::*;

#[derive(Serialize, Debug, Default)]
pub struct Summary {
    pub activities: Vec<Activity>,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>,
    pub usages: Vec<Usage>,
}

impl Summary {
    pub fn merge(self, new: Summary) -> Summary {
        let mut hash = SumHash::new(self);
        hash.merge(new);
        hash.collect()
    }

    pub fn first_act(&self) -> ActivityId {
        self.activities[0].id
    }
}

#[derive(Debug, Default)]
pub(crate) struct SumHash {
    activities: HashMap<ActivityId, Activity>,
    parts: HashMap<PartId, Part>,
    atts: HashMap<String, AttachmentDetail>,
    uses: HashMap<UsageId, Usage>,
}

impl SumHash {
    pub fn new(sum: Summary) -> Self {
        let mut hash = SumHash::default();
        hash.merge(sum);
        hash
    }

    pub fn merge(&mut self, ps: Summary) {
        for act in ps.activities {
            self.activities.insert(act.id, act);
        }
        for part in ps.parts {
            self.parts.insert(part.id, part);
        }
        for att in ps.attachments {
            self.atts.insert(att.idx(), att);
        }
        for usage in ps.usages {
            self.uses.insert(usage.id, usage);
        }
    }

    pub fn collect(self) -> Summary {
        Summary {
            activities: self.activities.into_values().collect(),
            parts: self.parts.into_values().collect(),
            attachments: self.atts.into_values().collect(),
            usages: self.uses.into_values().collect(),
        }
    }
}
