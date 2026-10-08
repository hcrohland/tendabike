//! This module contains the implementation of the `Summary` struct and its associated functions.
//!
//! `Summary` is the only summary form: nine id-keyed maps, one per entity kind, each
//! `HashMap<Id, Option<E>>`. A `Some(entity)` value is a live entity; a `None` value is a
//! **tombstone** marking the entity as deleted. It is the single payload type the wire carries
//! (ADR-0005, executable spec #446 §2). A custom `Serialize` impl renders it as a uniform JSON object with
//! stringified id keys and `null` for tombstones.

use serde::Serialize;
use serde::ser::{SerializeMap, SerializeStruct, Serializer};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign, SubAssign},
};

use crate::*;

/// The id-keyed map summary: nine maps, one per entity kind, keyed by the entity's id. A
/// `Some(entity)` value is a live entity; a `None` value is a tombstone marking the entity as
/// deleted. It is the single payload type the wire carries (ADR-0005, executable spec #446 §2).
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

/// Per-kind merge operators: `+=` upserts the entity into its own field as `Some`
/// (per-id last-wins, same rule as the `Summary` merge) and `-=` (single only) inserts a
/// `None` tombstone — the deletion representation (ADR-0005, executable spec #446 §2;
/// "tombstone" in `CONTEXT.md`). Two arms: one keys on the public `id` field, one on
/// `idx()` for `AttachmentDetail` (whose wire key is the `idx` string).
macro_rules! impl_summary_entity_ops {
    ($summary:ty, $field:ident, $entity:ty, $idfield:ident) => {
        // Upsert a single entity into its field as `Some` (per-id last-wins).
        impl AddAssign<$entity> for $summary {
            fn add_assign(&mut self, rhs: $entity) {
                self.$field.insert(rhs.$idfield, Some(rhs));
            }
        }

        // Upsert each element of a vector into its field as `Some` (per-id last-wins).
        impl AddAssign<Vec<$entity>> for $summary {
            fn add_assign(&mut self, rhs: Vec<$entity>) {
                for e in rhs {
                    self.$field.insert(e.$idfield, Some(e));
                }
            }
        }

        // Record a single entity as deleted: insert a `None` tombstone for its id.
        impl SubAssign<$entity> for $summary {
            fn sub_assign(&mut self, rhs: $entity) {
                self.$field.insert(rhs.$idfield, None);
            }
        }
    };
    ($summary:ty, $field:ident, $entity:ty) => {
        // Upsert a single entity into its field as `Some`, keyed by its `idx` wire key
        // (per-id last-wins).
        impl AddAssign<$entity> for $summary {
            fn add_assign(&mut self, rhs: $entity) {
                self.$field.insert(rhs.idx(), Some(rhs));
            }
        }

        // Upsert each element of a vector into its field as `Some`, keyed by its `idx`
        // wire key (per-id last-wins).
        impl AddAssign<Vec<$entity>> for $summary {
            fn add_assign(&mut self, rhs: Vec<$entity>) {
                for e in rhs {
                    self.$field.insert(e.idx(), Some(e));
                }
            }
        }

        // Record a single entity as deleted: insert a `None` tombstone for its `idx`
        // wire key.
        impl SubAssign<$entity> for $summary {
            fn sub_assign(&mut self, rhs: $entity) {
                self.$field.insert(rhs.idx(), None);
            }
        }
    };
}

// Nine-line table, one row per kind; each row generates the three impls above
// (`AddAssign<E>`, `AddAssign<Vec<E>>`, `SubAssign<E>`). `AttachmentDetail` is the
// only `idx()`-keyed kind; the other eight key on their public `id` field.
impl_summary_entity_ops!(Summary, activities, Activity, id);
impl_summary_entity_ops!(Summary, parts, Part, id);
impl_summary_entity_ops!(Summary, attachments, AttachmentDetail);
impl_summary_entity_ops!(Summary, usages, Usage, id);
impl_summary_entity_ops!(Summary, services, Service, id);
impl_summary_entity_ops!(Summary, plans, ServicePlan, id);
impl_summary_entity_ops!(Summary, part_notes, PartNote, id);
impl_summary_entity_ops!(Summary, shops, Shop, id);
impl_summary_entity_ops!(Summary, users, UserPublic, id);

// --- Live-entity accessors ---
//
// Written explicitly (no `macro_rules!`): the `get_` prefix cannot be spliced from the field
// name in `macro_rules!` (no stable `concat_idents!`), and the body is a single line, so a
// macro would add indirection without saving duplication.

impl Summary {
    /// All live [`Activity`] in this summary, as an owned `Vec` (cloned; order
    /// unspecified; borrowing, tombstones dropped).
    pub fn get_activities(&self) -> Vec<Activity> {
        self.activities.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`Part`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_parts(&self) -> Vec<Part> {
        self.parts.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`AttachmentDetail`] in this summary, as an owned `Vec` (cloned;
    /// order unspecified; borrowing, tombstones dropped).
    pub fn get_attachments(&self) -> Vec<AttachmentDetail> {
        self.attachments
            .values()
            .filter_map(|v| v.clone())
            .collect()
    }

    /// All live [`Usage`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_usages(&self) -> Vec<Usage> {
        self.usages.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`Service`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_services(&self) -> Vec<Service> {
        self.services.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`ServicePlan`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_plans(&self) -> Vec<ServicePlan> {
        self.plans.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`PartNote`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_part_notes(&self) -> Vec<PartNote> {
        self.part_notes.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`Shop`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_shops(&self) -> Vec<Shop> {
        self.shops.values().filter_map(|v| v.clone()).collect()
    }

    /// All live [`UserPublic`] in this summary, as an owned `Vec` (cloned; order unspecified;
    /// borrowing, tombstones dropped).
    pub fn get_users(&self) -> Vec<UserPublic> {
        self.users.values().filter_map(|v| v.clone()).collect()
    }

    /// Whether nothing was touched: all nine maps are empty. A tombstone
    /// counts as touched (a delete is something to deliver). The executor
    /// uses this to decide the push: an empty `Summary` pushes no stream
    /// frame (ADR-0005, executable spec #446 §4.5).
    pub fn is_empty(&self) -> bool {
        self.activities.is_empty()
            && self.parts.is_empty()
            && self.attachments.is_empty()
            && self.usages.is_empty()
            && self.services.is_empty()
            && self.plans.is_empty()
            && self.part_notes.is_empty()
            && self.shops.is_empty()
            && self.users.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Attachment, AttachmentDetail, MAX_TIME, PartId, PartTypeId, Usage, UsageId};
    use serde_json::json;
    use time::macros::datetime;

    fn usage(id: UsageId, count: i32) -> Usage {
        Usage {
            id,
            count,
            time: count,
            ..Default::default()
        }
    }

    /// Build an `AttachmentDetail` keyed by a distinct `idx` (part id `part`)
    /// through the public deserializing interface (`name`/`what` are private).
    fn detail(part: i32) -> AttachmentDetail {
        let att = Attachment::new(
            PartId::from(part),
            datetime!(2024-01-01 12:00 UTC),
            PartId::from(part + 1_000),
            PartTypeId::from_id(1),
            MAX_TIME,
        );
        let mut v = serde_json::to_value(att).unwrap();
        let obj = v.as_object_mut().unwrap();
        obj.insert("name".to_string(), json!(""));
        obj.insert("what".to_string(), json!(0));
        serde_json::from_value::<AttachmentDetail>(v).unwrap()
    }

    #[test]
    fn is_empty_only_when_no_kind_touched() {
        assert!(Summary::default().is_empty());
        // A tombstone is touched content: a delete delivers something.
        let mut s = Summary::default();
        s.usages.insert(UsageId::new(), None);
        assert!(!s.is_empty());
        let mut s = Summary::default();
        s.usages
            .insert(UsageId::new(), Some(usage(UsageId::new(), 1)));
        assert!(!s.is_empty());
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

    // --- `+=` / `-=` for a single entity and entity vectors (id-keyed kind) ---

    #[test]
    fn add_assign_single_upserts_last_wins() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s.usages.insert(id, Some(usage(id, 1))); // a pre-existing live entry
        s += usage(id, 2);
        // The upsert overwrote the pre-existing entry (per-id last-wins).
        assert_eq!(s.usages[&id].as_ref().unwrap().count, 2);

        let id2 = UsageId::new();
        s += usage(id2, 5);
        // And it creates the entry for a new id.
        assert_eq!(s.usages[&id2].as_ref().unwrap().count, 5);
        assert_eq!(s.usages.len(), 2);
    }

    #[test]
    fn add_assign_vec_upserts_each() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let mut s = Summary::default();
        s += vec![usage(id1, 1), usage(id2, 2)];
        assert_eq!(s.usages[&id1].as_ref().unwrap().count, 1);
        assert_eq!(s.usages[&id2].as_ref().unwrap().count, 2);
        assert_eq!(s.usages.len(), 2);
    }

    #[test]
    fn sub_assign_inserts_tombstone() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s -= usage(id, 1);
        // The id is present, but only as a `None` tombstone (no entity data).
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id], None);
    }

    #[test]
    fn upsert_then_delete_leaves_tombstone() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s += usage(id, 1);
        s -= usage(id, 1);
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id], None);
    }

    #[test]
    fn delete_then_upsert_leaves_live() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s -= usage(id, 1);
        s += usage(id, 2);
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id].as_ref().unwrap().count, 2);
    }

    // --- `+=` / `-=` for a single entity and entity vectors (idx-keyed kind) ---

    #[test]
    fn detail_add_assign_single_upserts_last_wins() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s.attachments.insert(key.clone(), Some(detail(1)));
        s += d;
        // Upserted under its `idx` wire key, overwriting the pre-existing entry.
        assert_eq!(
            s.attachments[&key].as_ref().unwrap().a.part_id,
            PartId::from(1)
        );

        let d2 = detail(2);
        let key2 = d2.idx();
        s += d2;
        assert_eq!(s.attachments.len(), 2);
        assert!(s.attachments.contains_key(&key2));
    }

    #[test]
    fn detail_add_assign_vec_upserts_each() {
        let d1 = detail(1);
        let d2 = detail(2);
        let key1 = d1.idx();
        let key2 = d2.idx();
        let mut s = Summary::default();
        s += vec![d1, d2];
        assert_eq!(s.attachments.len(), 2);
        assert!(s.attachments.contains_key(&key1));
        assert!(s.attachments.contains_key(&key2));
    }

    #[test]
    fn detail_sub_assign_inserts_tombstone() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s -= d;
        assert_eq!(s.attachments.len(), 1);
        assert_eq!(s.attachments[&key], None);
    }

    #[test]
    fn detail_upsert_then_delete_leaves_tombstone() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s += d.clone();
        s -= d;
        assert_eq!(s.attachments.len(), 1);
        assert_eq!(s.attachments[&key], None);
    }

    #[test]
    fn detail_delete_then_upsert_leaves_live() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s -= d;
        s += detail(1);
        assert_eq!(s.attachments.len(), 1);
        assert!(s.attachments[&key].is_some());
    }

    // --- Live-entity accessors ---

    #[test]
    fn get_accessors_return_live_only_and_borrow() {
        let live = UsageId::new();
        let gone = UsageId::new();
        let mut s = Summary::default();
        s.usages.insert(live, Some(usage(live, 1)));
        s.usages.insert(gone, None); // tombstone

        let live_key = detail(1).idx();
        let gone_key = detail(2).idx();
        s.attachments.insert(live_key.clone(), Some(detail(1)));
        s.attachments.insert(gone_key.clone(), None); // tombstone

        // Each accessor returns only the live entities, cloned into an owned `Vec`.
        let u = s.get_usages();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].id, live);
        assert_eq!(u[0].count, 1);
        let a = s.get_attachments();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].idx(), live_key);

        // Borrowing, not consuming: each map still holds both entries, incl. the tombstone.
        assert_eq!(s.usages.len(), 2);
        assert_eq!(s.usages[&live].as_ref().unwrap().count, 1);
        assert_eq!(s.usages[&gone], None);
        assert_eq!(s.attachments.len(), 2);
        assert!(s.attachments[&live_key].is_some());
        assert_eq!(s.attachments[&gone_key], None);
    }
}
