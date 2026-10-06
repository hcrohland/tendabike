//! This module contains the implementation of the `Summary` struct and its associated functions.
//!
//! `Summary` is the id-keyed map form of the summary: nine maps, one per entity kind, each
//! `HashMap<Id, Option<E>>`. A `Some(entity)` value is a live entity; a `None` value is a
//! **tombstone** marking the entity as deleted. This is the single payload type the wire will
//! carry (ADR-0005 §2). A custom `Serialize` impl renders it as a uniform JSON object with
//! stringified id keys and `null` for tombstones.
//!
//! `SummaryVec` is the previous `Vec`-based form, kept under a temporary name. It remains the
//! return type of all domain operations and the current wire shape until the wire flips to the
//! map form.

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeStruct, Serializer};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
};

use crate::*;

/// The id-keyed map form of the summary: nine maps, one per entity kind, keyed by the entity's
/// id. A `Some(entity)` value is a live entity; a `None` value is a tombstone marking the
/// entity as deleted. This is the single payload type the wire will carry (ADR-0005 §2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub activities: HashMap<ActivityId, Option<Activity>>,
    pub parts: HashMap<PartId, Option<Part>>,
    pub attachments: HashMap<String, Option<AttachmentDetail>>,
    pub usages: HashMap<UsageId, Option<Usage>>,
    pub services: HashMap<ServiceId, Option<Service>>,
    pub plans: HashMap<ServicePlanId, Option<ServicePlan>>,
    pub part_notes: HashMap<PartNoteId, Option<PartNote>>,
    pub shops: HashMap<ShopId, Option<Shop>>,
    pub users: HashMap<UserId, Option<UserPublic>>,
}

/// The previous `Vec`-based form of the summary, kept under a temporary name. It remains the
/// return type of all domain operations and the current wire shape until the wire flips to the
/// map form (`Summary`).
#[derive(Clone, Serialize, Debug, Default, PartialEq)]
pub struct SummaryVec {
    pub activities: Vec<Activity>,
    pub parts: Vec<Part>,
    pub attachments: Vec<AttachmentDetail>,
    pub usages: Vec<Usage>,
    pub services: Vec<Service>,
    pub plans: Vec<ServicePlan>,
    pub part_notes: Vec<PartNote>,
    pub shops: Vec<Shop>,
    pub users: Vec<UserPublic>,
}

/// Serializes an id-keyed map as a JSON object with stringified keys and `null` for `None`
/// (tombstone) values, so every collection shares one uniform wire shape (a naive derive would
/// give pair-arrays for the integer-keyed collections and objects for the string/UUID-keyed
/// ones).
struct IdKeyedMap<'a, K, E>(&'a HashMap<K, Option<E>>)
where
    K: Display,
    E: Serialize;

impl<K, E> Serialize for IdKeyedMap<'_, K, E>
where
    K: Display,
    E: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in self.0 {
            map.serialize_entry(&k.to_string(), v)?;
        }
        map.end()
    }
}

/// The wire shape: a uniform JSON object with stringified id keys and `null` for tombstones —
/// `{"parts": {"12": null, "13": {...}}, "services": {"<uuid>": {...}}}`.
impl Serialize for Summary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("Summary", 9)?;
        s.serialize_field("activities", &IdKeyedMap(&self.activities))?;
        s.serialize_field("parts", &IdKeyedMap(&self.parts))?;
        s.serialize_field("attachments", &IdKeyedMap(&self.attachments))?;
        s.serialize_field("usages", &IdKeyedMap(&self.usages))?;
        s.serialize_field("services", &IdKeyedMap(&self.services))?;
        s.serialize_field("plans", &IdKeyedMap(&self.plans))?;
        s.serialize_field("part_notes", &IdKeyedMap(&self.part_notes))?;
        s.serialize_field("shops", &IdKeyedMap(&self.shops))?;
        s.serialize_field("users", &IdKeyedMap(&self.users))?;
        s.end()
    }
}

// --- Conversions between the map form and the Vec form ---

/// Build the map from the `Vec` form: each entity is upserted as `Some`.
impl From<SummaryVec> for Summary {
    fn from(value: SummaryVec) -> Self {
        Summary {
            activities: value
                .activities
                .into_iter()
                .map(|x| (x.id, Some(x)))
                .collect(),
            parts: value.parts.into_iter().map(|x| (x.id, Some(x))).collect(),
            attachments: value
                .attachments
                .into_iter()
                .map(|x| (x.idx(), Some(x)))
                .collect(),
            usages: value.usages.into_iter().map(|x| (x.id, Some(x))).collect(),
            services: value
                .services
                .into_iter()
                .map(|x| (x.id, Some(x)))
                .collect(),
            plans: value.plans.into_iter().map(|x| (x.id, Some(x))).collect(),
            part_notes: value
                .part_notes
                .into_iter()
                .map(|x| (x.id, Some(x)))
                .collect(),
            shops: value.shops.into_iter().map(|x| (x.id, Some(x))).collect(),
            users: value.users.into_iter().map(|x| (x.id, Some(x))).collect(),
        }
    }
}

/// Build the `Vec` form from the map: live entities are kept, tombstones (`None`) are dropped
/// (the `Vec` form cannot represent deletions).
impl From<Summary> for SummaryVec {
    fn from(value: Summary) -> Self {
        SummaryVec {
            activities: value.activities.into_values().flatten().collect(),
            parts: value.parts.into_values().flatten().collect(),
            attachments: value.attachments.into_values().flatten().collect(),
            usages: value.usages.into_values().flatten().collect(),
            services: value.services.into_values().flatten().collect(),
            plans: value.plans.into_values().flatten().collect(),
            part_notes: value.part_notes.into_values().flatten().collect(),
            shops: value.shops.into_values().flatten().collect(),
            users: value.users.into_values().flatten().collect(),
        }
    }
}

// --- Merging ---

/// Per-id merge, last-wins: each id in `rhs` overwrites the entry in `self` (a `None`
/// tombstone overwrites a `Some`, deleting the entity). No precedence rule between `Some` and
/// `None` is needed: a hash holds one entry per id, and no operation composes a delete and an
/// upsert of the same id, so each id in a composed `Summary` is either upserted or deleted.
impl AddAssign<Summary> for Summary {
    fn add_assign(&mut self, rhs: Summary) {
        for (k, v) in rhs.activities {
            self.activities.insert(k, v);
        }
        for (k, v) in rhs.parts {
            self.parts.insert(k, v);
        }
        for (k, v) in rhs.attachments {
            self.attachments.insert(k, v);
        }
        for (k, v) in rhs.usages {
            self.usages.insert(k, v);
        }
        for (k, v) in rhs.services {
            self.services.insert(k, v);
        }
        for (k, v) in rhs.plans {
            self.plans.insert(k, v);
        }
        for (k, v) in rhs.part_notes {
            self.part_notes.insert(k, v);
        }
        for (k, v) in rhs.shops {
            self.shops.insert(k, v);
        }
        for (k, v) in rhs.users {
            self.users.insert(k, v);
        }
    }
}

/// Per-id merge, last-wins (the non-mutating form of [`AddAssign`]).
impl Add for Summary {
    type Output = Self;
    fn add(self, rhs: Summary) -> Self::Output {
        let mut out = self;
        out += rhs;
        out
    }
}

/// Accumulate a `Vec`-form summary into the map: each entity is upserted as `Some`.
impl AddAssign<SummaryVec> for Summary {
    fn add_assign(&mut self, rhs: SummaryVec) {
        *self += Summary::from(rhs);
    }
}

/// Accumulate a single part into the map (upserted as `Some`).
impl AddAssign<Part> for Summary {
    fn add_assign(&mut self, rhs: Part) {
        self.parts.insert(rhs.id, Some(rhs));
    }
}

/// Merge two `Vec`-form summaries (per-id, last-wins) — the previous `Summary` `Add`.
impl Add for SummaryVec {
    type Output = Self;
    fn add(self, rhs: SummaryVec) -> Self::Output {
        let mut map = Summary::from(self);
        map += rhs;
        map.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Usage, UsageId};
    use serde_json::json;

    fn usage(id: UsageId, count: i32) -> Usage {
        Usage {
            id,
            count,
            time: count,
            ..Default::default()
        }
    }

    #[test]
    fn add_summaries_vec() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let sum1 = SummaryVec {
            usages: vec![usage(id1, 1)],
            ..Default::default()
        };
        let sum2 = SummaryVec {
            usages: vec![usage(id2, 2)],
            ..Default::default()
        };
        let sum3 = SummaryVec {
            usages: vec![usage(id1, 3)],
            ..Default::default()
        };
        // Vec + Vec merges per-id, last-wins.
        let sum4 = sum1.clone() + sum2.clone();
        assert_eq!(sum4.usages.len(), 2);
        let sum4 = sum4 + sum3.clone();
        assert_eq!(sum4.usages.len(), 2);
        assert_eq!(sum4.usages.iter().find(|u| u.id == id1).unwrap().count, 3);
        assert_eq!(sum4.usages.iter().find(|u| u.id == id2).unwrap().count, 2);
    }

    #[test]
    fn map_merge_last_wins() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let mut a = Summary::default();
        a.usages.insert(id1, Some(usage(id1, 1)));
        a.usages.insert(id2, Some(usage(id2, 2)));
        let mut b = Summary::default();
        // b overwrites id1 with a tombstone and upserts id2.
        b.usages.insert(id1, None);
        b.usages.insert(id2, Some(usage(id2, 20)));
        let c = a + b;
        // id1 is deleted (tombstone won), id2 is the later upsert.
        assert_eq!(c.usages[&id1], None);
        assert_eq!(c.usages[&id2].as_ref().unwrap().count, 20);
        assert_eq!(c.usages.len(), 2);
    }

    #[test]
    fn vec_map_roundtrip() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let vec = SummaryVec {
            usages: vec![usage(id1, 1), usage(id2, 2)],
            ..Default::default()
        };
        let map = Summary::from(vec.clone());
        assert_eq!(map.usages[&id1].as_ref().unwrap().count, 1);
        assert_eq!(map.usages[&id2].as_ref().unwrap().count, 2);
        // Back to the Vec form: all live, no tombstones to drop. Element order is not
        // preserved (the map is unordered) — this matches the original `SumHash` behaviour,
        // so compare as a set, not by order.
        let round = SummaryVec::from(map);
        let round_counts: HashMap<UsageId, i32> =
            round.usages.into_iter().map(|u| (u.id, u.count)).collect();
        assert_eq!(round_counts.get(&id1), Some(&1));
        assert_eq!(round_counts.get(&id2), Some(&2));
        assert_eq!(round_counts.len(), 2);
    }

    #[test]
    fn wire_shape_stringified_keys_and_tombstones() {
        // The `IdKeyedMap` helper renders a map as a JSON object with stringified keys and
        // `null` for `None` — the case a naive derive breaks (integer keys would become
        // pair-arrays, string/UUID keys would become objects).
        let mut m: HashMap<i32, Option<i32>> = HashMap::new();
        m.insert(12, None); // tombstone
        m.insert(13, Some(42));
        assert_eq!(
            serde_json::to_value(super::IdKeyedMap(&m)).unwrap(),
            json!({"12": null, "13": 42})
        );

        // The full `Summary` is a uniform object: all nine collections are objects (never
        // pair-arrays), even the empty ones.
        let uid = UsageId::new();
        let mut map = Summary::default();
        map.usages.insert(uid, Some(usage(uid, 1)));
        let value = serde_json::to_value(&map).unwrap();
        for field in [
            "activities",
            "parts",
            "attachments",
            "usages",
            "services",
            "plans",
            "part_notes",
            "shops",
            "users",
        ] {
            assert!(value[field].is_object(), "{field} should be an object");
        }
        // Stringified UUID key; the entity's fields are present.
        assert_eq!(value["usages"][uid.to_string()]["count"], json!(1));
        // Empty collections serialize as empty objects, not arrays.
        assert_eq!(value["activities"], json!({}));
    }
}
