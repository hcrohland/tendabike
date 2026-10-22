//! This module contains the implementation of the `Summary` struct and its associated functions.
//!
//! `Summary` is the only summary form: nine id-keyed maps, one per entity kind, each an
//! [`IdKeyedMap`] hiding a `HashMap<Id, Option<E>>`. A `Some(entity)` value is a live entity; a
//! `None` value is a **tombstone** marking the entity as deleted. It is the single payload type
//! the wire carries (ADR-0005, executable spec #446 §2). Each map renders as a uniform JSON
//! object with stringified id keys and `null` for tombstones.

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};
use std::{
    cmp::PartialEq,
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    ops::{Add, AddAssign},
};

use crate::*;

/// An entity that carries its own map key: the id under which its [`IdKeyedMap`] entry lives.
pub trait IdKeyed {
    /// The map key type.
    type Key;

    /// This entity's key.
    fn key(&self) -> Self::Key;
}

/// The id-keyed map: a `HashMap<K, Option<E>>` hidden under a newtype, hosting every per-map
/// operation generically — [`upsert`](IdKeyedMap::upsert),
/// [`upsert_all`](IdKeyedMap::upsert_all), [`tombstone`](IdKeyedMap::tombstone),
/// [`live`](IdKeyedMap::live), [`merge`](IdKeyedMap::merge), [`len`](IdKeyedMap::len),
/// [`is_empty`](IdKeyedMap::is_empty), [`get`](IdKeyedMap::get),
/// [`is_tombstoned`](IdKeyedMap::is_tombstoned) — each written once, applying to all nine
/// [`Summary`] kinds. A `Some(entity)` value is a live entity; a `None` value is a tombstone
/// marking the entity as deleted (ADR-0005, executable spec #446 §2; "tombstone" and "live" in
/// `CONTEXT.md`). The inner `HashMap` is private and unreachable: every read and write goes
/// through the nine named ops.
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

impl<K, E> IdKeyedMap<K, E>
where
    K: Eq + Hash,
{
    /// Record an id as deleted: insert a `None` tombstone under the key — the deletion
    /// representation (ADR-0005, executable spec #446 §2). The tombstone entry carries no
    /// entity data ("tombstone" in `CONTEXT.md`), so the key is the whole input.
    pub fn tombstone(&mut self, k: K) {
        self.0.insert(k, None);
    }

    /// Entry count, tombstones included: a tombstone is an entry, and a delete is something
    /// to deliver.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are no entries at all. A tombstone-only map is **not** empty: the
    /// caller uses [`Summary::is_empty`] to decide the push, and a delete still delivers a
    /// frame (ADR-0005, executable spec #446 §4.5).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The live entity under `k`: `None` for a tombstoned or absent id (never a tombstone).
    pub fn get(&self, k: &K) -> Option<&E> {
        self.0.get(k).and_then(Option::as_ref)
    }

    /// Whether the entry under `k` exists and is a tombstone (absent ids are not tombstoned).
    pub fn is_tombstoned(&self, k: &K) -> bool {
        matches!(self.0.get(k), Some(None))
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
    /// The single part owner (spec #517): every part in the `Summary` shares
    /// one owner (a domain-layer invariant), so the first live part's owner is
    /// the unambiguous answer. `None` when there are no live parts. A
    /// read-only convenience for the executor's shop-context routing (the
    /// `push_to_user` target); it does not change the `Summary`'s shape.
    pub fn part_owner(&self) -> Option<UserId> {
        self.parts.live().first().map(|part| part.owner)
    }

    /// Whether nothing was touched: all nine maps are empty. A tombstone
    /// counts as touched (a delete is something to deliver). The caller
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
mod summary_tests;
