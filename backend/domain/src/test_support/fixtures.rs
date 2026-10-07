//! Test fixtures for domain entity tests.
//!
//! Provides shared helper functions and prepopulated MemStore scenarios
//! for testing entity operations across all domains.
//!
//! Built through the store's documented interface (issue #410): fixture
//! data is written with domain operations and store trait methods, and read
//! back through store trait methods — never through the store's private
//! state. All fixture times are fixed values, so two builds of the same
//! scenario produce identical data (see the determinism test at the bottom).
//!
//! One direct-write exception: `fixture_assembly` writes two overlapping
//! tire rows through `attachment_create`, a state the attach operation
//! never produces (see its comment).

use super::{AttachmentStore, MemStore, TestSession, part_type_ids};
use crate::MAX_TIME;
use crate::UserId;
use crate::attach_assembly;
use crate::{Attachment, OffsetDateTime, Part, PartId, TbResult};

// Re-export PartTypeId constants for tests (UPPERCASE per Rust conventions)
use part_type_ids::*;

/// Returns a test UserId (ID = 1).
pub fn test_user() -> UserId {
    UserId::from(1)
}

/// Returns a TestSession initialized with test_user().
pub fn test_session() -> TestSession {
    TestSession::new(test_user())
}

/// Create a basic part for testing.
///
/// Creates a part with minimal required fields using the TestSession
/// and stores it in the provided MemStore.
pub async fn fixture_basic_part(session: &TestSession, store: &mut MemStore) -> TbResult<Part> {
    Part::create(
        "Test Chain".to_string(),
        "Shimano".to_string(),
        "CN-M510".to_string(),
        CHAIN,
        None,
        sample_purchase_date(),
        session,
        store,
    )
    .await
}

/// Create a part and attach it to a gear.
///
/// Creates a basic part, then a new main bike, and attaches the part to it
/// at the sample attach time.
pub async fn fixture_attached_part(
    session: &TestSession,
    store: &mut MemStore,
) -> TbResult<(Part, Attachment)> {
    let part = fixture_basic_part(session, store).await?;
    let attachment = attach_test_part(session, store, part.clone()).await?;
    Ok((part, attachment))
}

/// Create a part and attach it to the specified gear.
///
/// Unlike `fixture_attached_part`, this accepts an existing gear Part
/// instead of creating a new bike internally.
pub async fn fixture_attached_part_to_gear(
    session: &TestSession,
    store: &mut MemStore,
    part: Part,
    gear_id: PartId,
) -> TbResult<Attachment> {
    attach_test_part_at(session, store, part, gear_id, sample_attach_time()).await
}

/// Create a part and attach it to the specified gear at a specific time.
pub async fn fixture_attached_part_at(
    session: &TestSession,
    store: &mut MemStore,
    part: Part,
    gear_id: PartId,
    attach_time: OffsetDateTime,
) -> TbResult<Attachment> {
    attach_test_part_at(session, store, part, gear_id, attach_time).await
}

/// Create an assembly with a main part and subparts attached.
///
/// Creates a front wheel (main_part) with two tires attached at the same
/// time. Uses BIKE → FRONT_WHEEL → TIRE hierarchy since subparts() relies on
/// type hooks. The main_part is attached with the attach operation; the tire
/// rows are written directly, because two tires on one wheel overlap in time
/// — a state attach never produces (see below).
pub async fn fixture_assembly(
    session: &TestSession,
    store: &mut MemStore,
    attach_time: time::OffsetDateTime,
) -> TbResult<(Part, Vec<Part>, Attachment)> {
    let main_part = Part::create(
        "Front Wheel".to_string(),
        "Zipp".to_string(),
        "404 Firecrest".to_string(),
        FRONT_WHEEL,
        None,
        sample_purchase_date(),
        session,
        store,
    )
    .await?;

    let subpart1 = Part::create(
        "Tire 1".to_string(),
        "Continental".to_string(),
        "Grand Prix 5000".to_string(),
        TIRE,
        None,
        sample_purchase_date(),
        session,
        store,
    )
    .await?;

    let subpart2 = Part::create(
        "Tire 2".to_string(),
        "Continental".to_string(),
        "Grand Prix 5000 S TR".to_string(),
        TIRE,
        None,
        sample_purchase_date() - time::Duration::days(10),
        session,
        store,
    )
    .await?;

    let gear = Part::create(
        "Main Bike Frame".to_string(),
        "TendaBike".to_string(),
        "Standard".to_string(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        session,
        store,
    )
    .await?;

    // Attach main_part (FRONT_WHEEL) to gear (BIKE)
    let main_hook = main_part
        .what
        .get()
        .map(|t| t.hooks.first().copied().unwrap_or(main_part.what))
        .unwrap_or(main_part.what);
    let _main_summary = attach_assembly(
        session,
        main_part.id,
        attach_time,
        gear.id,
        main_hook,
        false,
        store,
    )
    .await?;

    // The tire rows are written directly through the store interface — the
    // documented exception in this module (issue #410). Two tires on one
    // wheel at the same hook overlap in time; the attach operation never
    // produces such a state, it replaces the part already occupying the
    // hook (the state-space decision recorded on `attach_assembly`, issue
    // #407 — the database allows the overlap, attach does not create it).
    // Writing both rows keeps them coexisting.
    let front_wheel_id = main_part.id;
    let hook = TIRE
        .get()
        .ok()
        .and_then(|t| t.hooks.first().copied())
        .unwrap_or(TIRE);

    for subpart in [&subpart1, &subpart2] {
        let att = Attachment::new(subpart.id, attach_time, front_wheel_id, hook, MAX_TIME);
        store.attachment_create(att).await?;
    }

    // Read the main part's attachment back through the store's interface
    // rather than its internals (issue #410). attach_assembly always writes
    // one row for the part it attaches, so the lookup must succeed.
    let main_part_id = main_part.id;
    let main_attachment = store
        .attachments_all_by_part(main_part_id)
        .await?
        .into_iter()
        .min_by_key(|a| a.attached)
        .ok_or_else(|| {
            crate::Error::NotFound(format!(
                "no attachment row for part {main_part_id} after attach_assembly"
            ))
        })?;

    Ok((main_part, vec![subpart1, subpart2], main_attachment))
}

/// Create a timeline of sequential attachments for the same part/gear/hook.
///
/// Creates multiple attachment records at different times, forming a timeline
/// of install/remove cycles. Each attachment follows the previous one.
///
/// # Arguments
/// * `session` - Test session for authentication
/// * `store` - MemStore to store parts and attachments
/// * `parts` - Vector of (Part, gear Part, attachment_time) tuples
///
/// # Returns
/// Vector of Attachment records in chronological order
pub async fn fixture_timeline(
    session: &TestSession,
    store: &mut MemStore,
    parts: Vec<(Part, Part, OffsetDateTime)>,
) -> TbResult<Vec<Attachment>> {
    let mut attachments = Vec::new();

    for (part, gear, attach_time) in parts {
        let attachment = attach_test_part_at(session, store, part, gear.id, attach_time).await?;
        attachments.push(attachment);
    }

    Ok(attachments)
}

/// Create multiple parts attached to the same gear at different times.
///
/// Useful for testing concurrent attachment queries and part replacement scenarios.
pub async fn fixture_concurrent_parts(
    session: &TestSession,
    store: &mut MemStore,
) -> TbResult<(Part, Part, Attachment, Attachment)> {
    let _gear = Part::create(
        "Front Wheel".to_string(),
        "Zipp".to_string(),
        "404 Firecrest".to_string(),
        FRONT_WHEEL,
        None,
        sample_purchase_date() - time::Duration::days(180),
        session,
        store,
    )
    .await?;

    let part1 = Part::create(
        "Tire 1".to_string(),
        "Continental".to_string(),
        "Grand Prix 5000".to_string(),
        TIRE,
        None,
        sample_purchase_date() - time::Duration::days(90),
        session,
        store,
    )
    .await?;

    let part2 = Part::create(
        "Tire 2".to_string(),
        "Continental".to_string(),
        "Grand Prix 5000 S TR".to_string(),
        TIRE,
        None,
        sample_purchase_date() - time::Duration::days(30),
        session,
        store,
    )
    .await?;

    let att1 = attach_test_part(session, store, part1.clone()).await?;
    let att2 = attach_test_part(session, store, part2.clone()).await?;

    Ok((part1, part2, att1, att2))
}

// ─── Internal helper functions ────────────────────────────────────────────────

/// Returns a sample purchase date (fixed for deterministic tests).
pub fn sample_purchase_date() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// Create a new main bike and attach a part to it at the sample attach time.
async fn attach_test_part(
    session: &TestSession,
    store: &mut MemStore,
    part: Part,
) -> TbResult<Attachment> {
    let gear = fixture_bike(session, store).await?;
    attach_test_part_at(session, store, part, gear.id, sample_attach_time()).await
}

/// Attach a part to a specific gear at a specific time.
async fn attach_test_part_at(
    session: &TestSession,
    store: &mut MemStore,
    part: Part,
    gear_id: PartId,
    time: OffsetDateTime,
) -> TbResult<Attachment> {
    let hook = part
        .what
        .get()
        .map(|t| t.hooks.first().copied().unwrap_or(part.what))
        .unwrap_or(part.what);

    let summary = attach_assembly(
        session, part.id, time, gear_id, hook, false, // all = false for basic attachment
        store,
    )
    .await?;

    // Read the attachment back through the store's interface rather than its
    // internals (issue #410): the part's rows, earliest first.
    if let Some(part_obj) = summary.parts.values().flatten().next() {
        let atts = store.attachments_all_by_part(part_obj.id).await?;
        if let Some(att) = atts.into_iter().min_by_key(|a| a.attached) {
            return Ok(att);
        }
    }

    // Fallback: any attachment row for the part
    store
        .attachments_all_by_part(part.id)
        .await?
        .into_iter()
        .min_by_key(|a| a.attached)
        .ok_or_else(|| {
            crate::Error::NotFound(format!("Could not find attachment for part {}", part.id))
        })
}

/// Create the main bike frame/gear part.
pub async fn fixture_bike(session: &TestSession, store: &mut MemStore) -> TbResult<Part> {
    Part::create(
        "Main Bike".to_string(),
        "TendaBike".to_string(),
        "Standard Frame".to_string(),
        BIKE,
        None,
        sample_purchase_date() - time::Duration::days(365),
        session,
        store,
    )
    .await
}

/// Returns a sample attach time (30 days after the sample purchase date).
///
/// Fixed, like every other fixture time (issue #410): the suite has no wall
/// clock, so the time is an offset from the fixed sample purchase date, not
/// from the current date.
fn sample_attach_time() -> OffsetDateTime {
    sample_purchase_date() + time::Duration::days(30)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UsageId;
    use crate::test_support::StoreSnapshot;

    /// The prepopulated fixture is built with fixed times only (issue #410):
    /// two independent builds of the same scenario must yield identical data.
    ///
    /// Usage UUIDs are the one non-deterministic byte: the domain operations
    /// mint them via `Uuid::now_v7()` on every run (the same model the
    /// snapshot documents — "the structure is deterministic; only the UUID
    /// values differ between builds"). The comparison canonicalizes them to
    /// nil; every other field — times, ids, relationships, values — must
    /// match exactly.
    #[tokio::test]
    async fn prepopulated_fixture_is_deterministic_across_builds() -> TbResult<()> {
        async fn build() -> TbResult<StoreSnapshot> {
            let mut store = MemStore::prepopulated();
            let s = test_session();

            // every fixture function of this module, in one scenario
            let _ = fixture_assembly(&s, &mut store, sample_attach_time()).await?;
            let _ = fixture_attached_part(&s, &mut store).await?;
            let _ = fixture_concurrent_parts(&s, &mut store).await?;
            let bike = fixture_bike(&s, &mut store).await?;
            let part = fixture_basic_part(&s, &mut store).await?;
            let _ = fixture_attached_part_to_gear(&s, &mut store, part, bike.id).await?;

            Ok(store.snapshot())
        }

        let a = canonicalize_usage_uuids(build().await?);
        let b = canonicalize_usage_uuids(build().await?);

        assert_eq!(a, b);
        Ok(())
    }

    /// Strips the run-dependent usage UUIDs from a snapshot so two builds of
    /// the same scenario can be compared byte for byte (see
    /// `prepopulated_fixture_is_deterministic_across_builds`). Usage rows are
    /// compared as a value-sorted multiset once their ids are nilled.
    fn canonicalize_usage_uuids(mut snap: StoreSnapshot) -> StoreSnapshot {
        for p in &mut snap.parts {
            p.usage = UsageId::default();
        }
        for a in &mut snap.attachments {
            a.usage = UsageId::default();
        }
        for u in &mut snap.usages {
            u.id = UsageId::default();
        }
        snap.usages.sort_by(|x, y| {
            (x.time, x.distance, x.climb, x.descend, x.energy, x.count)
                .cmp(&(y.time, y.distance, y.climb, y.descend, y.energy, y.count))
        });
        snap
    }
}
