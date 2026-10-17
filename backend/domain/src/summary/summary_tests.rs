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
    s.usages.tombstone(UsageId::new());
    assert!(!s.is_empty());
    let mut s = Summary::default();
    s.usages.upsert(usage(UsageId::new(), 1));
    assert!(!s.is_empty());
}

#[test]
fn map_merge_last_wins() {
    let id1 = UsageId::new();
    let id2 = UsageId::new();
    let mut a = Summary::default();
    a.usages.upsert(usage(id1, 1));
    a.usages.upsert(usage(id2, 2));
    let mut b = Summary::default();
    // b overwrites id1 with a tombstone and upserts id2.
    b.usages.tombstone(id1);
    b.usages.upsert(usage(id2, 20));
    let c = a + b;
    // id1 is deleted (tombstone won), id2 is the later upsert.
    assert!(c.usages.is_tombstoned(&id1));
    assert_eq!(c.usages.get(&id2).unwrap().count, 20);
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
    map.usages.upsert(usage(uid, 1));
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
    s.usages.upsert(usage(id, 1)); // a pre-existing live entry
    s.usages.upsert(usage(id, 2));
    // The upsert overwrote the pre-existing entry (per-id last-wins).
    assert_eq!(s.usages.get(&id).unwrap().count, 2);

    let id2 = UsageId::new();
    s.usages.upsert(usage(id2, 5));
    // And it creates the entry for a new id.
    assert_eq!(s.usages.get(&id2).unwrap().count, 5);
    assert_eq!(s.usages.len(), 2);
}

#[test]
fn upsert_all_upserts_each() {
    let id1 = UsageId::new();
    let id2 = UsageId::new();
    let mut s = Summary::default();
    s.usages.upsert_all(vec![usage(id1, 1), usage(id2, 2)]);
    assert_eq!(s.usages.get(&id1).unwrap().count, 1);
    assert_eq!(s.usages.get(&id2).unwrap().count, 2);
    assert_eq!(s.usages.len(), 2);
}

#[test]
fn tombstone_inserts_tombstone() {
    let id = UsageId::new();
    let mut s = Summary::default();
    s.usages.tombstone(id);
    // The id is present, but only as a `None` tombstone (no entity data).
    assert_eq!(s.usages.len(), 1);
    assert!(s.usages.is_tombstoned(&id));
}

#[test]
fn upsert_then_delete_leaves_tombstone() {
    let id = UsageId::new();
    let mut s = Summary::default();
    s.usages.upsert(usage(id, 1));
    s.usages.tombstone(id);
    assert_eq!(s.usages.len(), 1);
    assert!(s.usages.is_tombstoned(&id));
}

#[test]
fn delete_then_upsert_leaves_live() {
    let id = UsageId::new();
    let mut s = Summary::default();
    s.usages.tombstone(id);
    s.usages.upsert(usage(id, 2));
    assert_eq!(s.usages.len(), 1);
    assert_eq!(s.usages.get(&id).unwrap().count, 2);
}

#[test]
fn get_and_is_tombstoned_absent_id() {
    let id = UsageId::new();
    let s = Summary::default();
    // An absent id is neither live nor tombstoned.
    assert!(s.usages.get(&id).is_none());
    assert!(!s.usages.is_tombstoned(&id));
    assert_eq!(s.usages.len(), 0);
    assert!(s.usages.is_empty());
}

// --- `upsert` / `upsert_all` / `tombstone` (idx-keyed kind) ---

#[test]
fn detail_upsert_single_last_wins() {
    let d = detail(1);
    let key = d.idx();
    let mut s = Summary::default();
    s.attachments.upsert(detail(1));
    s.attachments.upsert(d);
    // Upserted under its `idx` wire key, overwriting the pre-existing entry.
    assert_eq!(s.attachments.get(&key).unwrap().a.part_id, PartId::from(1));

    let d2 = detail(2);
    let key2 = d2.idx();
    s.attachments.upsert(d2);
    assert_eq!(s.attachments.len(), 2);
    assert!(s.attachments.get(&key2).is_some());
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
    assert!(s.attachments.get(&key1).is_some());
    assert!(s.attachments.get(&key2).is_some());
}

#[test]
fn detail_tombstone_inserts_tombstone() {
    let key = detail(1).idx();
    let mut s = Summary::default();
    s.attachments.tombstone(key.clone());
    assert_eq!(s.attachments.len(), 1);
    assert!(s.attachments.is_tombstoned(&key));
}

#[test]
fn detail_upsert_then_delete_leaves_tombstone() {
    let d = detail(1);
    let key = d.idx();
    let mut s = Summary::default();
    s.attachments.upsert(d);
    s.attachments.tombstone(key.clone());
    assert_eq!(s.attachments.len(), 1);
    assert!(s.attachments.is_tombstoned(&key));
}

#[test]
fn detail_delete_then_upsert_leaves_live() {
    let key = detail(1).idx();
    let mut s = Summary::default();
    s.attachments.tombstone(key.clone());
    s.attachments.upsert(detail(1));
    assert_eq!(s.attachments.len(), 1);
    assert!(s.attachments.get(&key).is_some());
}

// --- `live` ---

#[test]
fn live_returns_live_only_and_borrows() {
    let live = UsageId::new();
    let gone = UsageId::new();
    let mut s = Summary::default();
    s.usages.upsert(usage(live, 1));
    s.usages.tombstone(gone); // tombstone

    let live_key = detail(1).idx();
    let gone_key = detail(2).idx();
    s.attachments.upsert(detail(1));
    s.attachments.tombstone(gone_key.clone()); // tombstone

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
    assert_eq!(s.usages.get(&live).unwrap().count, 1);
    assert!(s.usages.is_tombstoned(&gone));
    assert_eq!(s.attachments.len(), 2);
    assert!(s.attachments.get(&live_key).is_some());
    assert!(s.attachments.is_tombstoned(&gone_key));
}
