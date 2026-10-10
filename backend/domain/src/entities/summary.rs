//! This module contains the implementation of the `Summary` struct and its associated functions.
//!
//! `Summary` is the only summary form: nine id-keyed maps, one per entity kind, each an
//! [`IdKeyedMap`] over a `HashMap<Id, Option<E>>`. A `Some(entity)` value is a live entity; a
//! `None` value is a **tombstone** marking the entity as deleted. It is the single payload type
//! the wire carries (ADR-0005, executable spec #446 §2). Each map renders as a uniform JSON
//! object with stringified id keys and `null` for tombstones.

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use std::{
    cmp::PartialEq,
    collections::hash_map::{self, HashMap},
    fmt::Display,
    hash::Hash,
    ops::{Add, AddAssign, Deref, DerefMut, Index},
};

use crate::*;

/// An entity that carries its own map key: the id under which its [`IdKeyedMap`] entry lives.
pub trait IdKeyed {
    /// The map key type.
    type Key;

    /// This entity's key.
    fn key(&self) -> Self::Key;
}

/// The id-keyed map: a `HashMap<K, Option<E>>` under a newtype, hosting every per-map
/// operation generically — [`upsert`](IdKeyedMap::upsert),
/// [`upsert_all`](IdKeyedMap::upsert_all), [`tombstone`](IdKeyedMap::tombstone),
/// [`live`](IdKeyedMap::live), [`merge`](IdKeyedMap::merge) — each written once, applying to
/// all nine [`Summary`] kinds. A `Some(entity)` value is a live entity; a `None` value is a
/// tombstone marking the entity as deleted (ADR-0005, executable spec #446 §2; "tombstone" in
/// `CONTEXT.md`).
#[derive(Clone, Debug, Serialize)]
#[serde(bound(serialize = "K: Display, E: Serialize"))]
pub struct IdKeyedMap<K, E>(
    #[serde(serialize_with = "serialize_id_keyed_map")] HashMap<K, Option<E>>,
);

/// Serializes the id-keyed map as a JSON object with stringified keys and `null` for `None`
/// (tombstone) values, so every collection shares one uniform wire shape (a naive derive would
/// give pair-arrays for the integer-keyed collections and objects for the string/UUID-keyed
/// ones).
fn serialize_id_keyed_map<K, E, S>(
    map: &HashMap<K, Option<E>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    K: Display,
    E: Serialize,
    S: Serializer,
{
    let mut ser_map = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        ser_map.serialize_entry(&k.to_string(), v)?;
    }
    ser_map.end()
}

impl<K, E> PartialEq for IdKeyedMap<K, E>
where
    K: Eq + Hash,
    E: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, E> Default for IdKeyedMap<K, E> {
    fn default() -> Self {
        Self(HashMap::default())
    }
}

// --- Shims so existing bare-`HashMap` field access compiles unchanged ---

impl<K, E> Deref for IdKeyedMap<K, E> {
    type Target = HashMap<K, Option<E>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, E> DerefMut for IdKeyedMap<K, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, E> Index<&K> for IdKeyedMap<K, E>
where
    K: Eq + Hash,
{
    type Output = Option<E>;

    fn index(&self, key: &K) -> &Self::Output {
        &self.0[key]
    }
}

impl<K, E> IntoIterator for IdKeyedMap<K, E> {
    type Item = (K, Option<E>);
    type IntoIter = hash_map::IntoIter<K, Option<E>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K, E> IntoIterator for &'a IdKeyedMap<K, E> {
    type Item = (&'a K, &'a Option<E>);
    type IntoIter = hash_map::Iter<'a, K, Option<E>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<K, E> PartialEq<HashMap<K, Option<E>>> for IdKeyedMap<K, E>
where
    K: Eq + Hash,
    E: PartialEq,
{
    fn eq(&self, other: &HashMap<K, Option<E>>) -> bool {
        self.0 == *other
    }
}

impl<K, E> IdKeyedMap<K, E>
where
    K: Eq + Hash,
    E: IdKeyed<Key = K>,
{
    /// Upsert one entity as `Some` under its key (per-id last-wins).
    pub fn upsert(&mut self, e: E) {
        self.0.insert(e.key(), Some(e));
    }

    /// Upsert each entity of `ents` as `Some` under its key (per-id last-wins).
    pub fn upsert_all(&mut self, ents: impl IntoIterator<Item = E>) {
        for e in ents {
            self.upsert(e);
        }
    }

    /// Record one entity as deleted: insert a `None` tombstone under its key — the deletion
    /// representation (ADR-0005, executable spec #446 §2).
    pub fn tombstone(&mut self, e: E) {
        self.0.insert(e.key(), None);
    }

    /// Per-id merge, last-wins: each entry of `other` overwrites the entry with the same key
    /// (a `None` tombstone overwrites a `Some`, deleting the entity).
    pub fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

impl<K, E> IdKeyedMap<K, E>
where
    E: Clone,
{
    /// All live entities in this map, as an owned `Vec` (cloned; order unspecified; borrowing,
    /// tombstones dropped).
    pub fn live(&self) -> Vec<E> {
        self.0.values().filter_map(|v| v.clone()).collect()
    }
}

/// The id-keyed map summary: nine [`IdKeyedMap`]s, one per entity kind, keyed by the entity's
/// id (its `idx` wire key for [`AttachmentDetail`]). A `Some(entity)` value is a live entity; a
/// `None` value is a tombstone marking the entity as deleted. It is the single payload type the
/// wire carries (ADR-0005, executable spec #446 §2). The wire shape: a uniform JSON object with
/// stringified id keys and `null` for tombstones — `{"parts": {"12": null, "13": {...}},
/// "services": {"<uuid>": {...}}}`.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Summary {
    pub activities: IdKeyedMap<ActivityId, Activity>,
    pub parts: IdKeyedMap<PartId, Part>,
    pub attachments: IdKeyedMap<String, AttachmentDetail>,
    pub usages: IdKeyedMap<UsageId, Usage>,
    pub services: IdKeyedMap<ServiceId, Service>,
    pub plans: IdKeyedMap<ServicePlanId, ServicePlan>,
    pub part_notes: IdKeyedMap<PartNoteId, PartNote>,
    pub shops: IdKeyedMap<ShopId, Shop>,
    pub users: IdKeyedMap<UserId, UserPublic>,
}

// --- Merging ---

/// Per-id merge, last-wins: each id in `rhs` overwrites the entry in `self` (a `None`
/// tombstone overwrites a `Some`, deleting the entity). No precedence rule between `Some` and
/// `None` is needed: a hash holds one entry per id, and no operation composes a delete and an
/// upsert of the same id, so each id in a composed `Summary` is either upserted or deleted.
impl AddAssign<Summary> for Summary {
    fn add_assign(&mut self, rhs: Summary) {
        self.activities.merge(rhs.activities);
        self.parts.merge(rhs.parts);
        self.attachments.merge(rhs.attachments);
        self.usages.merge(rhs.usages);
        self.services.merge(rhs.services);
        self.plans.merge(rhs.plans);
        self.part_notes.merge(rhs.part_notes);
        self.shops.merge(rhs.shops);
        self.users.merge(rhs.users);
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

impl Summary {
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
        // The `IdKeyedMap` newtype renders a map as a JSON object with stringified keys and
        // `null` for `None` — the case a naive derive breaks (integer keys would become
        // pair-arrays, string/UUID keys would become objects).
        let mut m: HashMap<i32, Option<i32>> = HashMap::new();
        m.insert(12, None); // tombstone
        m.insert(13, Some(42));
        assert_eq!(
            serde_json::to_value(IdKeyedMap(m)).unwrap(),
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

    // --- `upsert` / `upsert_all` / `tombstone` (id-keyed kind) ---

    #[test]
    fn upsert_single_last_wins() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s.usages.insert(id, Some(usage(id, 1))); // a pre-existing live entry
        s.usages.upsert(usage(id, 2));
        // The upsert overwrote the pre-existing entry (per-id last-wins).
        assert_eq!(s.usages[&id].as_ref().unwrap().count, 2);

        let id2 = UsageId::new();
        s.usages.upsert(usage(id2, 5));
        // And it creates the entry for a new id.
        assert_eq!(s.usages[&id2].as_ref().unwrap().count, 5);
        assert_eq!(s.usages.len(), 2);
    }

    #[test]
    fn upsert_all_upserts_each() {
        let id1 = UsageId::new();
        let id2 = UsageId::new();
        let mut s = Summary::default();
        s.usages.upsert_all(vec![usage(id1, 1), usage(id2, 2)]);
        assert_eq!(s.usages[&id1].as_ref().unwrap().count, 1);
        assert_eq!(s.usages[&id2].as_ref().unwrap().count, 2);
        assert_eq!(s.usages.len(), 2);
    }

    #[test]
    fn tombstone_inserts_tombstone() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s.usages.tombstone(usage(id, 1));
        // The id is present, but only as a `None` tombstone (no entity data).
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id], None);
    }

    #[test]
    fn upsert_then_delete_leaves_tombstone() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s.usages.upsert(usage(id, 1));
        s.usages.tombstone(usage(id, 1));
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id], None);
    }

    #[test]
    fn delete_then_upsert_leaves_live() {
        let id = UsageId::new();
        let mut s = Summary::default();
        s.usages.tombstone(usage(id, 1));
        s.usages.upsert(usage(id, 2));
        assert_eq!(s.usages.len(), 1);
        assert_eq!(s.usages[&id].as_ref().unwrap().count, 2);
    }

    // --- `upsert` / `upsert_all` / `tombstone` (idx-keyed kind) ---

    #[test]
    fn detail_upsert_single_last_wins() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s.attachments.insert(key.clone(), Some(detail(1)));
        s.attachments.upsert(d);
        // Upserted under its `idx` wire key, overwriting the pre-existing entry.
        assert_eq!(
            s.attachments[&key].as_ref().unwrap().a.part_id,
            PartId::from(1)
        );

        let d2 = detail(2);
        let key2 = d2.idx();
        s.attachments.upsert(d2);
        assert_eq!(s.attachments.len(), 2);
        assert!(s.attachments.contains_key(&key2));
    }

    #[test]
    fn detail_upsert_all_upserts_each() {
        let d1 = detail(1);
        let d2 = detail(2);
        let key1 = d1.idx();
        let key2 = d2.idx();
        let mut s = Summary::default();
        s.attachments.upsert_all(vec![d1, d2]);
        assert_eq!(s.attachments.len(), 2);
        assert!(s.attachments.contains_key(&key1));
        assert!(s.attachments.contains_key(&key2));
    }

    #[test]
    fn detail_tombstone_inserts_tombstone() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s.attachments.tombstone(d);
        assert_eq!(s.attachments.len(), 1);
        assert_eq!(s.attachments[&key], None);
    }

    #[test]
    fn detail_upsert_then_delete_leaves_tombstone() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s.attachments.upsert(d.clone());
        s.attachments.tombstone(d);
        assert_eq!(s.attachments.len(), 1);
        assert_eq!(s.attachments[&key], None);
    }

    #[test]
    fn detail_delete_then_upsert_leaves_live() {
        let d = detail(1);
        let key = d.idx();
        let mut s = Summary::default();
        s.attachments.tombstone(d);
        s.attachments.upsert(detail(1));
        assert_eq!(s.attachments.len(), 1);
        assert!(s.attachments[&key].is_some());
    }

    // --- `live` ---

    #[test]
    fn live_returns_live_only_and_borrows() {
        let live = UsageId::new();
        let gone = UsageId::new();
        let mut s = Summary::default();
        s.usages.insert(live, Some(usage(live, 1)));
        s.usages.insert(gone, None); // tombstone

        let live_key = detail(1).idx();
        let gone_key = detail(2).idx();
        s.attachments.insert(live_key.clone(), Some(detail(1)));
        s.attachments.insert(gone_key.clone(), None); // tombstone

        // Each map returns only its live entities, cloned into an owned `Vec`.
        let u = s.usages.live();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].id, live);
        assert_eq!(u[0].count, 1);
        let a = s.attachments.live();
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
