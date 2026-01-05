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

#[derive(Clone, Serialize, Debug, Default, PartialEq)]
pub struct Summary {
    pub activities: Vec<Activity>,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>,
    pub usages: Vec<Usage>,
    pub services: Vec<Service>,
    pub plans: Vec<ServicePlan>,
    pub shops: Vec<ShopWithOwner>,
}

impl From<SumHash> for Summary {
    fn from(value: SumHash) -> Self {
        Summary {
            activities: value.activities.into_values().collect(),
            parts: value.parts.into_values().collect(),
            attachments: value.atts.into_values().collect(),
            usages: value.uses.into_values().collect(),
            services: value.servs.into_values().collect(),
            plans: value.plans.into_values().collect(),
            shops: value.shops.into_values().collect(),
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
    servs: HashMap<ServiceId, Service>,
    plans: HashMap<ServicePlanId, ServicePlan>,
    shops: HashMap<ShopId, ShopWithOwner>,
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
        for x in rhs.services {
            self.servs.insert(x.id, x);
        }
        for x in rhs.plans {
            self.plans.insert(x.id, x);
        }
        for x in rhs.shops {
            self.shops.insert(x.id, x);
        }
    }
}

impl AddAssign<Part> for SumHash {
    fn add_assign(&mut self, rhs: Part) {
        self.parts.insert(rhs.id, rhs);
    }
}

#[cfg(test)]
mod tests {
    use crate::{SumHash, Summary, Usage, UsageId};

    #[test]
    fn add_summaries() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let usage1 = Usage {
            id: id1,
            count: 1,
            time: 1,
            ..Default::default()
        };
        let usage2 = Usage {
            id: id2,
            count: 2,
            time: 2,
            ..Default::default()
        };
        let usage3 = Usage {
            id: id1,
            count: 3,
            time: 3,
            ..Default::default()
        };
        let sum1 = Summary {
            usages: vec![usage1],
            ..Default::default()
        };
        let sum2 = Summary {
            usages: vec![usage2],
            ..Default::default()
        };
        let sum3 = Summary {
            usages: vec![usage3],
            ..Default::default()
        };
        let mut hash1 = SumHash::from(sum1.clone());
        hash1 += sum1.clone();
        assert_eq!(&sum1, &hash1.into());
        let sum4 = sum1 + sum2.clone();
        assert!(sum4.usages.len() == 2);
        let sum4 = sum4 + sum3.clone();
        let hash = SumHash::from(sum4);
        assert_eq!(hash.uses[&id1], sum3.usages[0]);
        assert_eq!(hash.uses[&id2], sum2.usages[0]);
    }
}
