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
use std::{
    collections::HashMap,
    ops::{Add, AddAssign},
};

use crate::*;

#[derive(Serialize, Debug, Default)]
pub struct Summary {
    pub activities: Vec<Activity>,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>,
    pub usages: Vec<Usage>,
}

impl From<SumHash> for Summary {
    fn from(value: SumHash) -> Self {
        Summary {
            activities: value.activities.into_values().collect(),
            parts: value.parts.into_values().collect(),
            attachments: value.atts.into_values().collect(),
            usages: value.uses.into_values().collect(),
        }
    }
}
impl Add for Summary {
    type Output = Self;

    fn add(self, rhs: Summary) -> Self::Output {
        let mut hash = SumHash::from(self);
        hash += rhs;
        hash.into()
    }
}

#[derive(Debug, Default)]
pub(crate) struct SumHash {
    activities: HashMap<ActivityId, Activity>,
    parts: HashMap<PartId, Part>,
    atts: HashMap<String, AttachmentDetail>,
    uses: HashMap<UsageId, Usage>,
}

impl From<Summary> for SumHash {
    fn from(value: Summary) -> Self {
        let mut hash = SumHash::default();
        hash += value;
        hash
    }
}

impl AddAssign<Summary> for SumHash {
    fn add_assign(&mut self, rhs: Summary) {
        for x in rhs.activities {
            self.activities.insert(x.id, x);
        }
        for x in rhs.parts {
            self.parts.insert(x.id, x);
        }
        for x in rhs.attachments {
            self.atts.insert(x.idx(), x);
        }
        for x in rhs.usages {
            self.uses.insert(x.id, x);
        }
    }
}
