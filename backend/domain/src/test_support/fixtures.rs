//! Test fixtures for domain entity tests.
//!
//! Provides shared helper functions and prepopulated MemStore scenarios
//! for testing entity operations across all domains.

use super::{AttachmentStore, MemStore, TestSession, part_type_ids};
use crate::MAX_TIME;
use crate::UserId;
use crate::attach_assembly;
use crate::{Attachment, OffsetDateTime, Part, PartId, TbResult, Usage, UsageId};
use uuid::Uuid;

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
        "Test part".to_string(),
        session,
        store,
    )
    .await
}

/// Create a part and attach it to a gear.
///
/// Creates a basic part, then attaches it to the main bike frame (gear)
/// at the given time using the attach_assembly function.
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
/// Creates a front wheel (main_part) with tires (subparts) attached at the same time.
/// Uses BIKE → FRONT_WHEEL → TIRE hierarchy since subparts() relies on type hooks.
/// The main_part is attached to gear, and subparts are directly inserted into the store
/// to avoid attach_assembly's replacement logic for same-type attachments.
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
        "Main assembly part".to_string(),
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
        "Subpart 1".to_string(),
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
        "Subpart 2".to_string(),
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
        "Main gear".to_string(),
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

    // Insert subpart attachments directly into the store to bypass attach_assembly's
    // replacement logic. When attaching two TIREs (same type, same hook) via attach_assembly,
    // the second would replace the first. Direct insertion ensures both coexist.
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

    // Find and return the main part's attachment
    let main_part_id = main_part.id;
    if let Some(main_attachment) = store
        .attachments
        .values()
        .find(|a| a.part_id == main_part_id)
    {
        Ok((main_part, vec![subpart1, subpart2], main_attachment.clone()))
    } else {
        Ok((
            main_part,
            vec![subpart1, subpart2],
            Attachment::new(main_part_id, attach_time, gear.id, main_hook, MAX_TIME),
        ))
    }
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
        "Test gear".to_string(),
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
        "First tire".to_string(),
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
        "Second tire".to_string(),
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

/// Attach a part to the main bike frame using attach_assembly.
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
    let usage = Usage::new(UsageId::from(Uuid::from_u128(1)));
    let _ = Usage::update_vec(&[usage], store).await?;

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

    // Extract attachment from summary - use the first affected part's attachment
    if let Some(part_obj) = summary.parts.first() {
        // Re-query to get the attachment
        let all_attachments: Vec<Attachment> = store
            .attachments
            .values()
            .filter(|a| a.part_id == part_obj.id)
            .cloned()
            .collect();

        if let Some(att) = all_attachments.into_iter().min_by_key(|a| a.attached) {
            return Ok(att);
        }
    }

    // Fallback: return any attachment for the part
    let first_att = store.attachments.values().find(|a| a.part_id == part.id);

    match first_att {
        Some(att) => Ok(*att),
        None => Err(crate::Error::NotFound(format!(
            "Could not find attachment for part {}",
            part.id
        ))),
    }
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
        "Main bike frame".to_string(),
        session,
        store,
    )
    .await
}

/// Returns a sample attach time (30 days ago).
fn sample_attach_time() -> OffsetDateTime {
    OffsetDateTime::now_utc() - time::Duration::days(30)
}
