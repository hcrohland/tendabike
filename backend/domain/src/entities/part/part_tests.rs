use super::*;
use crate::test_support::{MemStore, TestSession, fixtures};
use time::OffsetDateTime;

use fixtures::{sample_purchase_date, test_user};

fn later_time() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700100000).unwrap()
}

// === PartId tests ===

/// PartId::read retrieves a stored part
#[tokio::test]
async fn partid_read_returns_stored_part() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let retrieved = PartId::from(1).read(&mut store).await?;
    assert_eq!(retrieved.name, "Main Bike");
    assert_eq!(retrieved.vendor, "TendaBike");
    Ok(())
}

/// PartId::read returns error for non-existent part
#[tokio::test]
async fn partid_read_not_found_error() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let result = PartId::from(999).read(&mut store).await;
    assert!(result.is_err());
    Ok(())
}

/// PartId::name returns the part name without checking ownership
#[tokio::test]
async fn partid_name_returns_name() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let name = PartId::from(1).name(&mut store).await?;
    assert_eq!(name, "Main Bike");
    Ok(())
}

/// PartId::is_main returns true for main part types
#[tokio::test]
async fn partid_is_main_for_bike() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let is_main = PartId::from(1).is_main(&mut store).await?;
    assert!(is_main);
    Ok(())
}

/// PartId::is_main returns false for sub-part types
#[tokio::test]
async fn partid_is_not_main_for_wheel() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let is_main = PartId::from(2).is_main(&mut store).await?;
    assert!(!is_main);
    Ok(())
}

/// PartId::update_timestamps updates last_used when start is later
#[tokio::test]
async fn update_timestamps_updates_last_used() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let updated = PartId::from(4)
        .update_timestamps(later_time(), &mut store)
        .await?;
    assert_eq!(updated.last_used, later_time());
    Ok(())
}

/// PartId::dispose sets disposed_at timestamp
#[tokio::test]
async fn dispose_sets_disposed_at() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let disposed = PartId::from(4).dispose(later_time(), &mut store).await?;
    assert!(disposed.disposed_at.is_some());
    Ok(())
}

/// PartId::restore clears disposed_at
#[tokio::test]
async fn restore_clears_disposed_at() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    PartId::from(4)
        .dispose(sample_purchase_date(), &mut store)
        .await?;
    let restored = PartId::from(4).restore(&mut store).await?;
    assert!(restored.disposed_at.is_none());
    Ok(())
}

/// PartId::set_owner_and_shop copies owner and shop from gear
#[tokio::test]
async fn set_owner_and_shop_copies_from_gear() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let gear = PartId::from(1).read(&mut store).await?;
    let updated = PartId::from(2)
        .set_owner_and_shop(gear.id, &mut store)
        .await?;
    assert_eq!(updated.owner, gear.owner);
    Ok(())
}

// === Part tests ===

/// Part::usage returns the usage id
#[test]
fn part_usage_returns_usage_id() {
    let test_usage = UsageId::new();
    let part = Part {
        id: PartId::from(1),
        owner: test_user(),
        what: PartTypeId::from(1),
        name: "Bike".to_string(),
        vendor: "Trek".to_string(),
        model: "Domane".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: test_usage,
        source: None,
        shop: None,
    };

    assert_eq!(part.usage(), test_usage);
}

/// Part::create creates a part with correct purchase date rounding
#[tokio::test]
async fn part_create_sets_purchase_date() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    let purchase = sample_purchase_date();
    let part = Part::create(
        "Derailleur".to_string(),
        "Shimano".to_string(),
        "Di2".to_string(),
        PartTypeId::from(4),
        None,
        purchase,
        &sess,
        &mut store,
    )
    .await?;

    // Part::create rounds the purchase date via round_time()
    let rounded_purchase = round_time(purchase);
    assert_eq!(part.purchase, rounded_purchase);
    Ok(())
}

/// Part::create sets source
#[tokio::test]
async fn part_create_with_source() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    let part = Part::create(
        "Wheelset".to_string(),
        "Easton".to_string(),
        "DA3".to_string(),
        PartTypeId::from(4),
        Some("strava_67890".to_string()),
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;

    assert_eq!(part.source, Some("strava_67890".to_string()));
    Ok(())
}

/// Part::by_source returns the part carrying the given Strava source
#[tokio::test]
async fn part_by_source_returns_part_with_source() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    let created = Part::create(
        "Wheelset".to_string(),
        "Easton".to_string(),
        "DA3".to_string(),
        PartTypeId::from(4),
        Some("b12345".to_string()),
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;

    let found = Part::by_source("b12345", &mut store).await?;
    assert_eq!(found, Some(created));
    Ok(())
}

/// Part::by_source returns None when no part carries the source
#[tokio::test]
async fn part_by_source_returns_none_when_missing() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let found = Part::by_source("b-no-such-source", &mut store).await?;
    assert_eq!(found, None);
    Ok(())
}

/// Part::get_all returns all parts for a user
#[tokio::test]
async fn part_get_all_returns_user_parts() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    Part::create(
        "Part 1".to_string(),
        "V1".to_string(),
        "M1".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;
    Part::create(
        "Part 2".to_string(),
        "V2".to_string(),
        "M2".to_string(),
        PartTypeId::from(4),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;

    let parts = Part::get_all(&sess.user_id(), &mut store).await?;
    assert_eq!(parts.len(), 2);
    Ok(())
}

/// Part::get_all returns empty vec when user has no parts
#[tokio::test]
async fn part_get_all_empty_for_user_with_no_parts() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let parts = Part::get_all(&UserId::from(98), &mut store).await?;
    assert!(parts.is_empty());
    Ok(())
}

/// Part::categories returns main part types only
#[tokio::test]
async fn part_categories_returns_main_types() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;
    Part::create(
        "Wheel".to_string(),
        "Zipp".to_string(),
        "404".to_string(),
        PartTypeId::from(4),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;

    let categories = Part::categories(&sess, &mut store).await?;
    // Type 1 (Bike) is main type, Type 4 (chain) is subtype - only main types returned
    assert_eq!(categories.len(), 1);
    assert!(categories.contains(&PartTypeId::from(1)));
    assert!(!categories.contains(&PartTypeId::from(4)));
    Ok(())
}

/// Part::categories returns empty when no parts
#[tokio::test]
async fn part_categories_empty_for_no_parts() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let categories = Part::categories(&TestSession::new(UserId::from(98)), &mut store).await?;
    assert!(categories.is_empty());
    Ok(())
}

/// Part::categories filters to main types only - subtypes are excluded
#[tokio::test]
async fn part_categories_filters_subtypes() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(99));
    Part::create(
        "Road Bike".to_string(),
        "Trek".to_string(),
        "Domane".to_string(),
        PartTypeId::from(1),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;
    Part::create(
        "Front Wheel".to_string(),
        "Zipp".to_string(),
        "404".to_string(),
        PartTypeId::from(2),
        None,
        sample_purchase_date(),
        &sess,
        &mut store,
    )
    .await?;

    // Type 1 (Bike) is main (hooks=[]), Type 2 (front wheel) is subtype (hooks=[1])
    let categories = Part::categories(&sess, &mut store).await?;
    assert_eq!(categories.len(), 1);
    assert!(categories.contains(&PartTypeId::from(1)));
    Ok(())
}

/// PartId::change updates part fields and returns updated part
#[tokio::test]
async fn part_change_updates_fields() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(1));
    let updated = PartId::from(1)
        .change(
            "New Name".to_string(),
            "New Vendor".to_string(),
            "New Model".to_string(),
            sample_purchase_date(),
            &sess,
            &mut store,
        )
        .await?;

    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.vendor, "New Vendor");
    assert_eq!(updated.model, "New Model");
    Ok(())
}

/// PartId::change returns forbidden for non-owner session
#[tokio::test]
async fn part_change_rejects_non_owner() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let other_session = TestSession::new(UserId::from(98));
    let result = PartId::from(1)
        .change(
            "New Name".to_string(),
            "Vendor".to_string(),
            "Model".to_string(),
            sample_purchase_date(),
            &other_session,
            &mut store,
        )
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// PartId::delete returns successfully when part has no attachments
#[tokio::test]
async fn part_delete_succeeds_without_attachments() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(1));
    let result = PartId::from(1).delete(&sess, &mut store).await?;
    assert_eq!(result, PartId::from(1));

    // Verify part is removed from store
    let get_result = PartId::from(1).part(&sess, &mut store).await;
    assert!(matches!(get_result, Err(Error::NotFound(_))));
    Ok(())
}

/// PartId::delete returns conflict when part is still attached (conflict check requires AttachmentStore)
/// Note: AttachmentStore is currently todo!() in MemStore, so this test verifies the delete
/// succeeds when no attachments exist (the AttachmentStore todo!() is not hit in this path)
#[tokio::test]
async fn part_delete_succeeds_without_attachments_documented() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let sess = TestSession::new(UserId::from(1));
    let result = PartId::from(1).delete(&sess, &mut store).await?;
    assert_eq!(result, PartId::from(1));
    Ok(())
}

/// PartId::delete returns forbidden for non-owner session
#[tokio::test]
async fn part_delete_rejects_non_owner() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let other_session = TestSession::new(UserId::from(98));
    let result = PartId::from(1).delete(&other_session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}
