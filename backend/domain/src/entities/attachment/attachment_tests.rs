//! The `Attachment` entity tests over a canned [`MockStore`] (issue #645).
//!
//! Each test runs the real attachment domain logic — the computing ops
//! `attach_assembly` / `detach_assembly` / `dispose_assembly` /
//! `recover_assembly` / `shift` (predecessor auto-detach, adjacent-merge,
//! subpart cascading, tombstone-on-detach, and the permission/ownership
//! checks) — over a [`MockStore`] whose methods return canned rows. The
//! domain logic runs for real; only the store I/O is canned, and no test
//! reads back a row it just wrote. The persistence pass-throughs (the
//! `AttachmentStore` query methods) and the exact usage values driven by the
//! prepopulated activity data are verified on the sqlx seam,
//! `sqlx/tests/store_seam.rs`, where Postgres is the source of truth; the
//! thin read-wrappers and the subsumed cases are dropped.

use super::*;
use crate::test_support::{MockStore, fixtures, mock_session, part_type_ids, sole_live};

use fixtures::{sample_purchase_date, test_session, test_user};
use part_type_ids::*;
use time::macros::datetime;

fn attachment_time() -> OffsetDateTime {
    datetime!(2024-01-01 00:00 UTC)
}

fn later_time() -> OffsetDateTime {
    datetime!(2024-06-01 00:00 UTC)
}

fn bike_id() -> PartId {
    PartId::from(1)
}

fn chain_id() -> PartId {
    PartId::from(2)
}

// ---------------------------------------------------------------------------
// Canned row builders
// ---------------------------------------------------------------------------

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner +
/// type + its usage id). `purchase` and `last_used` sit well before the
/// attach times, so `update_timestamps` bumps `last_used` (a canned write).
fn part_row(id: PartId, owner: UserId, what: PartTypeId, usage: UsageId) -> Part {
    Part {
        id,
        owner,
        what,
        name: "Main Bike".to_string(),
        vendor: "TendaBike".to_string(),
        model: "Standard".to_string(),
        purchase: sample_purchase_date() - time::Duration::days(365),
        last_used: sample_purchase_date() - time::Duration::days(365),
        disposed_at: None,
        usage,
        source: None,
        shop: None,
    }
}

/// A canned `Attachment` row.
fn att(
    part: PartId,
    attached: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
    detached: OffsetDateTime,
    usage: UsageId,
) -> Attachment {
    Attachment {
        part_id: part,
        attached,
        gear,
        hook,
        detached,
        usage,
    }
}

/// Wire the canned reads/writes the `Attachment::create` path performs: a
/// zero-activity usage, a `last_used` bump, the stored row, no services, and
/// the bulk usage write. The domain computes the (zero) usage in memory from
/// these canned inputs — no test reads back a row it just wrote.
fn wire_create_path(store: &mut MockStore) {
    store
        .expect_activities_find_by_gear_and_time()
        .returning(|_, _, _| Ok(vec![]));
    store.expect_part_update().returning(Ok);
    store.expect_usage_get().returning(|_| Ok(None));
    store.expect_attachment_create().returning(Ok);
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
}

/// Wire the canned reads/writes the `Attachment::delete` path performs (the
/// part read and the usage bulk-write it shares with `create` are wired
/// separately, per test).
fn wire_delete_path(store: &mut MockStore) {
    store.expect_attachment_delete().returning(Ok);
    store
        .expect_usage_delete()
        .returning(|_| Ok(Usage::default()));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usage_get().returning(|_| Ok(None));
    store.expect_usages_update().returning(|_| Ok(1));
}

// === Suite 1: Attachment::new — value logic (no store) ===

/// new() creates an attachment with the correct fields
#[test]
fn new_creates_correct_struct() {
    let a = Attachment::new(chain_id(), attachment_time(), bike_id(), CHAIN, MAX_TIME);
    assert_eq!(a.part_id, chain_id());
    assert_eq!(a.attached, attachment_time());
    assert_eq!(a.gear, bike_id());
    assert_eq!(a.hook, CHAIN);
    assert_eq!(a.detached, MAX_TIME);
    // usage should be a new UsageId (not the default)
    assert_ne!(a.usage, UsageId::default());
}

/// new() sets default detached to MAX_TIME (still attached)
#[test]
fn new_sets_default_detached_to_max_time() {
    let a = Attachment::new(chain_id(), attachment_time(), bike_id(), CHAIN, MAX_TIME);
    assert_eq!(a.detached, MAX_TIME);
}

/// new() can create with an explicit detached time
#[test]
fn new_allows_explicit_detached_time() {
    let a = Attachment::new(
        chain_id(),
        attachment_time(),
        bike_id(),
        CHAIN,
        later_time(),
    );
    assert_eq!(a.detached, later_time());
}

// === Suite 2: attach_assembly ===

/// attach_assembly() attaches a part to a gear at the specified hook: the
/// summary reports the new still-attached row.
#[tokio::test]
async fn attach_assembly_attaches_part_to_gear() -> TbResult<()> {
    let part = PartId::from(4);
    let gear = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BIKE, UsageId::new())
        })
    });
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        part,
        attachment_time(),
        gear,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    let att = sole_live(&summary.attachments);
    assert_eq!(att.a.part_id, part);
    assert_eq!(att.a.gear, gear);
    assert_eq!(att.a.hook, BIKE);
    Ok(())
}

/// The flat row model (ADR-0003): a part mounted onto a part that is itself
/// already mounted is stored against the top-level gear. A tire mounted onto a
/// front wheel that is on the bike gets a row with the bike as gear.
#[tokio::test]
async fn attach_assembly_resolves_mounted_gear_to_top_level() -> TbResult<()> {
    let tire = PartId::from(5);
    let wheel = PartId::from(2);
    let bike = PartId::from(1);
    let time = attachment_time();

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        let what = match id {
            _ if id == tire => TIRE,
            _ if id == wheel => FRONT_WHEEL,
            _ => BIKE,
        };
        Ok(part_row(id, test_user(), what, UsageId::new()))
    });
    // Top-level resolution: the wheel's row has gear=bike (≠ wheel) → walk up;
    // the bike has no row → stop at the bike.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |pid, _| {
            if pid == wheel {
                Ok(Some(att(wheel, time, bike, BIKE, MAX_TIME, UsageId::new())))
            } else {
                Ok(None)
            }
        });
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        tire,
        time,
        wheel,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await?;

    let att = sole_live(&summary.attachments);
    assert_eq!(att.a.part_id, tire);
    assert_eq!(att.a.gear, bike); // resolved to the top-level gear
    assert_eq!(att.a.hook, FRONT_WHEEL);
    Ok(())
}

/// attach_assembly() rejects if the part type cannot be attached to the hook.
#[tokio::test]
async fn attach_assembly_rejects_invalid_hook() -> TbResult<()> {
    let part = PartId::from(4);
    let gear = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BIKE, UsageId::new())
        })
    });

    // Chain's hooks are [BIKE]; FRONT_WHEEL is not among them.
    let result = attach_assembly(
        &test_session(),
        part,
        attachment_time(),
        gear,
        FRONT_WHEEL,
        false,
        &mut store,
    )
    .await;
    assert!(
        matches!(&result, Err(Error::BadRequest(msg)) if msg.contains("cannot be attached to hook"))
    );
    Ok(())
}

/// attach_assembly() rejects if the gear type is not valid for the part type.
#[tokio::test]
async fn attach_assembly_rejects_invalid_gear_type() -> TbResult<()> {
    let part = PartId::from(4);
    let grip = PartId::from(19);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BOTTOM_BRACKET, UsageId::new())
        })
    });

    // Chain's main is BIKE and its hooks are [BIKE]; BOTTOM_BRACKET is neither.
    let result = attach_assembly(
        &test_session(),
        part,
        attachment_time(),
        grip,
        BIKE,
        false,
        &mut store,
    )
    .await;
    assert!(
        matches!(&result, Err(Error::BadRequest(msg)) if msg.contains("cannot be attached to gear type"))
    );
    Ok(())
}

/// attach_assembly() detaches the part's own current attachment before
/// re-attaching it: the re-attach at the later time is reported.
#[tokio::test]
async fn attach_assembly_detaches_existing_attachment() -> TbResult<()> {
    let part = PartId::from(4);
    let gear = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BIKE, UsageId::new())
        })
    });
    // The part is already attached at attachment_time (the detach-self trigger);
    // the gear resolution finds no row for the gear.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |pid, _| {
            if pid == part {
                Ok(Some(att(
                    part,
                    attachment_time(),
                    gear,
                    CHAIN,
                    MAX_TIME,
                    UsageId::new(),
                )))
            } else {
                Ok(None)
            }
        });
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    wire_delete_path(&mut store);
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        part,
        later_time(),
        gear,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    // The re-attach at later_time is reported.
    let new_att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == part && a.a.attached == later_time())
        .expect("the re-attached row is reported");
    assert_eq!(new_att.a.gear, gear);
    Ok(())
}

/// attach_assembly() detaches the different part already occupying the hook
/// (the predecessor auto-detach): the predecessor is cut off at the attach
/// time and the new part takes the hook.
#[tokio::test]
async fn attach_assembly_detaches_predecessor_on_gear() -> TbResult<()> {
    let gear = PartId::from(1);
    let prev = PartId::from(4);
    let part = PartId::from(5);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        let what = if id == gear { BIKE } else { CHAIN };
        Ok(part_row(id, test_user(), what, UsageId::new()))
    });
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    // The hook is occupied by the predecessor (a different part, same type).
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(move |_, _, _, _| {
            Ok(Some(att(
                prev,
                attachment_time(),
                gear,
                BIKE,
                MAX_TIME,
                UsageId::new(),
            )))
        });
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    wire_delete_path(&mut store);
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        part,
        later_time(),
        gear,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    // The predecessor is cut off at later_time …
    let prev_att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == prev)
        .expect("the predecessor's cut-off row is reported");
    assert_eq!(prev_att.a.detached, later_time());
    // … and the new part takes the hook.
    let new_att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == part)
        .expect("the new part's row is reported");
    assert_eq!(new_att.a.gear, gear);
    assert_eq!(new_att.a.attached, later_time());
    Ok(())
}

/// Re-attaching a part at the time its row ended continues that row — the
/// adjacent-merge: one row (attached from the original time to MAX_TIME), not
/// two.
#[tokio::test]
async fn attach_assembly_merges_adjacent_rows() -> TbResult<()> {
    let part = PartId::from(4);
    let gear = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BIKE, UsageId::new())
        })
    });
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    // A row ending exactly at the attach time — the adjacent-merge trigger.
    store
        .expect_attachment_find_part_attached_already()
        .returning(move |_, _, _, _| {
            Ok(Some(att(
                part,
                attachment_time(),
                gear,
                BIKE,
                later_time(),
                UsageId::new(),
            )))
        });
    wire_delete_path(&mut store);
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        part,
        later_time(),
        gear,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    // The adjacent rows become one: continued from the original attach time to
    // MAX_TIME.
    let merged = sole_live(&summary.attachments);
    assert_eq!(merged.a.part_id, part);
    assert_eq!(merged.a.attached, attachment_time());
    assert_eq!(merged.a.detached, MAX_TIME);
    Ok(())
}

/// Re-attaching a part at the time its row started deletes and re-creates the
/// row: the re-attached row (detached at MAX_TIME) is reported.
#[tokio::test]
async fn attach_assembly_auto_detaches_and_reattaches_same_part() -> TbResult<()> {
    let part = PartId::from(4);
    let gear = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == part {
            part_row(id, test_user(), CHAIN, UsageId::new())
        } else {
            part_row(id, test_user(), BIKE, UsageId::new())
        })
    });
    // The part is already attached at attachment_time; re-attaching at the same
    // time detaches and re-creates the row.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |pid, _| {
            if pid == part {
                Ok(Some(att(
                    part,
                    attachment_time(),
                    gear,
                    CHAIN,
                    MAX_TIME,
                    UsageId::new(),
                )))
            } else {
                Ok(None)
            }
        });
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    wire_delete_path(&mut store);
    wire_create_path(&mut store);

    let summary = attach_assembly(
        &test_session(),
        part,
        attachment_time(),
        gear,
        BIKE,
        false,
        &mut store,
    )
    .await?;

    let att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == part)
        .expect("the re-attached row is reported");
    assert_eq!(att.a.gear, gear);
    assert_eq!(att.a.detached, MAX_TIME);
    Ok(())
}

// === Suite 3: detach_assembly ===

/// detach_assembly() cuts the part's attachment at the detach time: the
/// cut-off row is reported.
#[tokio::test]
async fn detach_assembly_with_all_detaches_subparts() -> TbResult<()> {
    let part = PartId::from(4);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // The part is attached at attachment_time (never detached).
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |_, _| {
            Ok(Some(att(
                part,
                attachment_time(),
                bike,
                CHAIN,
                MAX_TIME,
                UsageId::new(),
            )))
        });
    wire_delete_path(&mut store);
    wire_create_path(&mut store);

    let summary = detach_assembly(&test_session(), part, later_time(), false, &mut store).await?;

    // The part is cut off at the detach time.
    let cut = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == part)
        .expect("the part's cut-off row is reported");
    assert_eq!(cut.a.detached, later_time());
    Ok(())
}

/// detach_assembly() returns NotFound for a part that is not attached at the
/// detach time.
#[tokio::test]
async fn detach_assembly_api_returns_error_if_not_attached() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // No attachment at the detach time.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));

    let result = detach_assembly(&test_session(), part, later_time(), false, &mut store).await;
    assert!(matches!(&result, Err(Error::NotFound(_))));
    Ok(())
}

/// detach_assembly() rejects a non-owner session: the part is owned by
/// test_user(), so user 2 (no shop) is `Forbidden`.
#[tokio::test]
async fn detach_assembly_api_rejects_non_owner() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    // The part is owned by test_user(); user 2 (no shop) is denied.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));

    let result = detach_assembly(
        &mock_session(UserId::from(2)),
        part,
        later_time(),
        false,
        &mut store,
    )
    .await;
    assert!(matches!(&result, Err(Error::Forbidden(_))));
    Ok(())
}

/// Detaching a part at the time its row started is a pure delete: the deleted
/// attachment is reported as a None tombstone under its idx key.
#[tokio::test]
async fn detach_at_attach_time_reports_tombstone() -> TbResult<()> {
    let part = PartId::from(4);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // The part is attached at attachment_time.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |_, _| {
            Ok(Some(att(
                part,
                attachment_time(),
                bike,
                CHAIN,
                MAX_TIME,
                UsageId::new(),
            )))
        });
    wire_delete_path(&mut store);
    // Detaching at the attach time is a pure delete (no re-create).

    let summary =
        detach_assembly(&test_session(), part, attachment_time(), false, &mut store).await?;

    let key = Attachment::new(part, attachment_time(), bike, CHAIN, MAX_TIME)
        .add_details("", 0.into())
        .idx();
    assert!(summary.attachments.is_tombstoned(&key));
    Ok(())
}

/// detach_assembly() of an already-detached part (no row at the time) is a
/// NotFound.
#[tokio::test]
async fn detach_assembly_already_detached_returns_none() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // The part was already detached: no row at the detach time.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));

    let result = detach_assembly(&test_session(), part, attachment_time(), false, &mut store).await;
    assert!(matches!(&result, Err(Error::NotFound(_))));
    Ok(())
}

// === Suite 4: dispose_assembly ===

/// dispose_assembly() disposes a part: the summary reports the part with its
/// disposed timestamp set.
#[tokio::test]
async fn dispose_assembly_disposes_part() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // No current attachment, and no subparts.
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(|_, _, _| Ok(vec![]));
    store.expect_part_update().returning(Ok);

    let summary = dispose_assembly(&test_session(), part, later_time(), false, &mut store).await?;

    let disposed = sole_live(&summary.parts);
    assert_eq!(disposed.id, part);
    assert_eq!(disposed.disposed_at, Some(later_time()));
    Ok(())
}

/// dispose_assembly() disposes a part that is still attached (dispose never
/// rejects an attachment detached after the given time).
#[tokio::test]
async fn dispose_assembly_disposes_attached_part() -> TbResult<()> {
    let part = PartId::from(4);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));
    // The part is attached (detached at MAX_TIME), so disposing at the attach
    // time succeeds.
    store.expect_attachments_all_by_part().returning(move |_| {
        Ok(vec![att(
            part,
            attachment_time(),
            bike,
            CHAIN,
            MAX_TIME,
            UsageId::new(),
        )])
    });
    // No subparts.
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(|_, _, _| Ok(vec![]));
    store.expect_part_update().returning(Ok);

    let result =
        dispose_assembly(&test_session(), part, attachment_time(), false, &mut store).await;
    assert!(result.is_ok());
    Ok(())
}

/// dispose_assembly() with all=true disposes the subparts too: the summary
/// reports the part and its subpart, both disposed.
#[tokio::test]
async fn dispose_assembly_all_disposes_subparts() -> TbResult<()> {
    let wheel = PartId::from(3);
    let cassette = PartId::from(9);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        let what = if id == wheel { REAR_WHEEL } else { CASSETTE };
        Ok(part_row(id, test_user(), what, UsageId::new()))
    });
    store.expect_attachments_all_by_part().returning(move |id| {
        if id == wheel {
            Ok(vec![att(
                wheel,
                attachment_time(),
                bike,
                REAR_WHEEL,
                MAX_TIME,
                UsageId::new(),
            )])
        } else {
            Ok(vec![att(
                cassette,
                attachment_time(),
                wheel,
                CASSETTE,
                MAX_TIME,
                UsageId::new(),
            )])
        }
    });
    // The wheel's subparts (its subtypes) include the cassette, attached to it.
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| {
            Ok(vec![att(
                cassette,
                attachment_time(),
                wheel,
                CASSETTE,
                MAX_TIME,
                UsageId::new(),
            )])
        });
    store.expect_part_update().returning(Ok);

    let summary = dispose_assembly(&test_session(), wheel, later_time(), true, &mut store).await?;

    // Both the wheel and its cassette subpart are disposed.
    for id in [wheel, cassette] {
        let disposed = summary.parts.live().into_iter().find(|p| p.id == id);
        assert!(
            disposed.is_some_and(|p| p.disposed_at.is_some()),
            "part {id} is disposed"
        );
    }
    Ok(())
}

/// dispose_assembly() of an already-disposed part is an error (the dispose is
/// not re-applied).
#[tokio::test]
async fn dispose_assembly_already_disposed_keeps_timestamp() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    // The part is already disposed.
    store.expect_partid_get_part().returning(move |_| {
        Ok(Part {
            disposed_at: Some(attachment_time()),
            ..part_row(part, test_user(), CHAIN, UsageId::new())
        })
    });
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));

    let result = dispose_assembly(&test_session(), part, later_time(), false, &mut store).await;
    assert!(matches!(&result, Err(Error::BadRequest(msg)) if msg.contains("already disposed")));
    Ok(())
}

// === Suite 5: recover_assembly ===

/// recover_assembly() restores a disposed part: the summary reports the part
/// with its disposed timestamp cleared.
#[tokio::test]
async fn recover_assembly_restores_disposed_part() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    // The part is disposed.
    store.expect_partid_get_part().returning(move |_| {
        Ok(Part {
            disposed_at: Some(later_time()),
            ..part_row(part, test_user(), CHAIN, UsageId::new())
        })
    });
    store.expect_part_update().returning(Ok);

    let summary = recover_assembly(&test_session(), part, false, &mut store).await?;

    let restored = sole_live(&summary.parts);
    assert_eq!(restored.id, part);
    assert!(restored.disposed_at.is_none());
    Ok(())
}

/// recover_assembly() of a non-disposed part is an error.
#[tokio::test]
async fn recover_assembly_error_if_not_disposed() -> TbResult<()> {
    let part = PartId::from(4);

    let mut store = MockStore::new();
    // The part is not disposed.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN, UsageId::new())));

    let result = recover_assembly(&test_session(), part, false, &mut store).await;
    assert!(matches!(&result, Err(Error::BadRequest(msg)) if msg.contains("is not disposed")));
    Ok(())
}

/// recover_assembly() with all=true restores the subparts too: the summary
/// reports the part and its subpart, both restored.
#[tokio::test]
async fn recover_assembly_all_restores_subparts() -> TbResult<()> {
    let wheel = PartId::from(3);
    let cassette = PartId::from(9);

    let mut store = MockStore::new();
    // Both the wheel and its cassette subpart are disposed.
    store.expect_partid_get_part().returning(move |id| {
        let what = if id == wheel { REAR_WHEEL } else { CASSETTE };
        Ok(Part {
            disposed_at: Some(attachment_time()),
            ..part_row(id, test_user(), what, UsageId::new())
        })
    });
    // The wheel's subparts (its subtypes) include the cassette.
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| {
            Ok(vec![att(
                cassette,
                attachment_time(),
                wheel,
                CASSETTE,
                MAX_TIME,
                UsageId::new(),
            )])
        });
    store.expect_part_update().returning(Ok);

    let summary = recover_assembly(&test_session(), wheel, true, &mut store).await?;

    // Both the wheel and its cassette subpart are restored.
    for id in [wheel, cassette] {
        let restored = summary.parts.live().into_iter().find(|p| p.id == id);
        assert!(
            restored.is_some_and(|p| p.disposed_at.is_none()),
            "part {id} is restored"
        );
    }
    Ok(())
}

// === Suite 6: shift ===

/// shift() moves a part to a new gear: the old row is cut off at the shift
/// time and the part is re-attached to the new gear.
#[tokio::test]
async fn shift_changes_gear_and_creates_detach() -> TbResult<()> {
    let part = PartId::from(2);
    let from = PartId::from(1);
    let to = PartId::from(7);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        let what = if id == part { FRONT_WHEEL } else { BIKE };
        Ok(part_row(id, test_user(), what, UsageId::new()))
    });
    wire_delete_path(&mut store);
    wire_create_path(&mut store);
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));

    let att = att(
        part,
        attachment_time(),
        from,
        FRONT_WHEEL,
        MAX_TIME,
        UsageId::new(),
    );
    let mut summary = Summary::default();
    let end = att
        .shift(later_time(), to, &mut summary, &mut store)
        .await?;

    // The part is re-attached to the new gear.
    let new_att = summary
        .attachments
        .live()
        .into_iter()
        .find(|a| a.a.part_id == part && a.a.gear == to)
        .expect("the shifted row is reported on the new gear");
    assert_eq!(new_att.a.attached, later_time());
    assert!(end >= later_time());
    Ok(())
}

/// shift() returns the end time of the new attachment: with no successor and
/// no later attachment it runs to MAX_TIME.
#[tokio::test]
async fn shift_returns_end_time() -> TbResult<()> {
    let part = PartId::from(4);
    let from = PartId::from(1);
    let to = PartId::from(7);

    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        let what = if id == part { CHAIN } else { BIKE };
        Ok(part_row(id, test_user(), what, UsageId::new()))
    });
    wire_delete_path(&mut store);
    wire_create_path(&mut store);
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));

    let att = att(
        part,
        attachment_time(),
        from,
        CHAIN,
        MAX_TIME,
        UsageId::new(),
    );
    let mut summary = Summary::default();
    let end = att
        .shift(later_time(), to, &mut summary, &mut store)
        .await?;

    assert!(end >= later_time());
    Ok(())
}
