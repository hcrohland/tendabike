//! The `Part`/`PartId` domain logic over a canned [`MockStore`] (issue #640).
//!
//! Each mock-runnable test runs the real domain operation — dispose/restore,
//! `by_source` lookup, `is_main` derivation from the static `PARTTYPES` table,
//! ownership/shop assignment with checks, purchase-date rounding, and source
//! attach — over a [`MockStore`] whose methods return canned rows. The domain
//! logic runs for real; only the store I/O is canned. The one pure value-logic
//! test (`Part::usage` field projection) needs no store at all. The `PartId::
//! read`/`name` lossless roundtrip and the `RowNotFound` mapping rest on the
//! sqlx seam suite (`sqlx/tests/store_seam.rs`), where Postgres is the source
//! of truth.

use super::*;
use crate::test_support::{
    MockStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    mock_session,
    part_type_ids::{BIKE, CHAIN, FRONT_WHEEL},
};
use time::OffsetDateTime;

fn later_time() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700100000).unwrap()
}

/// A canned `Part` row: the minimal shape the domain logic needs (id, owner,
/// type).
fn part_row(id: PartId, owner: UserId, what: PartTypeId) -> Part {
    Part {
        id,
        owner,
        what,
        name: "Main Bike".to_string(),
        vendor: "TendaBike".to_string(),
        model: "Standard".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    }
}

// === PartId: is_main (the static PARTTYPES table) ===

/// `PartId::is_main` is true for a main part type (the bike: `hooks=[]`).
#[tokio::test]
async fn partid_is_main_for_bike() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user(), BIKE)));
    let is_main = PartId::from(1).is_main(&mut store).await?;
    assert!(is_main);
    Ok(())
}

/// `PartId::is_main` is false for a sub-part type (the front wheel: `hooks=[1]`).
#[tokio::test]
async fn partid_is_not_main_for_wheel() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(2), test_user(), FRONT_WHEEL)));
    let is_main = PartId::from(2).is_main(&mut store).await?;
    assert!(!is_main);
    Ok(())
}

/// `PartId::owner` returns the part's owner without an ownership check.
#[tokio::test]
async fn partid_owner_returns_owner() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user(), BIKE)));
    let owner = PartId::from(1).owner(&mut store).await?;
    assert_eq!(owner, test_user());
    Ok(())
}

// === Part: shop registration / bulk delete (ADR-0010) ===

/// `Part::for_shop` reads the shop's parts via the store.
#[tokio::test]
async fn part_for_shop_delegates_to_store() -> TbResult<()> {
    let mut store = MockStore::new();
    let bike = part_row(PartId::from(1), test_user(), BIKE);
    let bike_for_assert = bike.clone();
    store
        .expect_part_get_for_shop()
        .times(1)
        .returning(move |_| Ok(vec![bike.clone()]));
    let parts = Part::for_shop(ShopId::from(7), &mut store).await?;
    assert_eq!(parts, vec![bike_for_assert]);
    Ok(())
}

/// `Part::register_to_shop` delegates to the store.
#[tokio::test]
async fn part_register_to_shop_delegates_to_store() -> TbResult<()> {
    let mut store = MockStore::new();
    let bike = part_row(PartId::from(1), test_user(), BIKE);
    let bike_for_assert = bike.clone();
    store
        .expect_parts_register_shop()
        .times(1)
        .returning(move |_, _| Ok(vec![bike.clone()]));
    let parts = Part::register_to_shop(ShopId::from(7), vec![PartId::from(1)], &mut store).await?;
    assert_eq!(parts, vec![bike_for_assert]);
    Ok(())
}

/// `Part::unregister_from_shop` delegates to the store.
#[tokio::test]
async fn part_unregister_from_shop_delegates_to_store() -> TbResult<()> {
    let mut store = MockStore::new();
    let bike = part_row(PartId::from(1), test_user(), BIKE);
    let bike_for_assert = bike.clone();
    store
        .expect_parts_unregister_shop()
        .times(1)
        .returning(move |_| Ok(vec![bike.clone()]));
    let parts = Part::unregister_from_shop(vec![PartId::from(1)], &mut store).await?;
    assert_eq!(parts, vec![bike_for_assert]);
    Ok(())
}

/// `Part::delete_many` delegates to the store.
#[tokio::test]
async fn part_delete_many_delegates_to_store() -> TbResult<()> {
    let mut store = MockStore::new();
    let bike = part_row(PartId::from(1), test_user(), BIKE);
    store
        .expect_parts_delete()
        .times(1)
        .returning(move |_| Ok(1));
    let n = Part::delete_many(&[bike], &mut store).await?;
    assert_eq!(n, 1);
    Ok(())
}

// === PartId: update_timestamps ===

/// `PartId::update_timestamps` bumps last_used when start is later.
#[tokio::test]
async fn update_timestamps_updates_last_used() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(4), test_user(), CHAIN)));
    store.expect_part_update().returning(Ok);
    let updated = PartId::from(4)
        .update_timestamps(later_time(), &mut store)
        .await?;
    assert_eq!(updated.last_used, later_time());
    Ok(())
}

// === PartId: dispose / restore ===

/// `PartId::dispose` sets the disposed_at timestamp.
#[tokio::test]
async fn dispose_sets_disposed_at() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(4), test_user(), CHAIN)));
    store.expect_part_update().returning(Ok);
    let disposed = PartId::from(4).dispose(later_time(), &mut store).await?;
    assert!(disposed.disposed_at.is_some());
    Ok(())
}

/// `PartId::restore` clears disposed_at.
#[tokio::test]
async fn restore_clears_disposed_at() -> TbResult<()> {
    let mut store = MockStore::new();
    let disposed = Part {
        disposed_at: Some(sample_purchase_date()),
        ..part_row(PartId::from(4), test_user(), CHAIN)
    };
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(disposed.clone()));
    store.expect_part_update().returning(Ok);
    let restored = PartId::from(4).restore(&mut store).await?;
    assert!(restored.disposed_at.is_none());
    Ok(())
}

// === PartId: set_owner_and_shop ===

/// `PartId::set_owner_and_shop` copies owner and shop from the gear.
#[tokio::test]
async fn set_owner_and_shop_copies_from_gear() -> TbResult<()> {
    let gear = part_row(PartId::from(1), test_user(), BIKE);
    let gear_owner = gear.owner;
    let gear_id = gear.id;
    let mut store = MockStore::new();
    store.expect_partid_get_part().returning(move |id| {
        if id == PartId::from(2) {
            Ok(part_row(PartId::from(2), UserId::from(2), FRONT_WHEEL))
        } else {
            Ok(gear.clone())
        }
    });
    store.expect_part_update().returning(Ok);
    let updated = PartId::from(2)
        .set_owner_and_shop(gear_id, &mut store)
        .await?;
    assert_eq!(updated.owner, gear_owner);
    Ok(())
}

// === Part: usage (pure value logic, no store) ===

/// `Part::usage` returns the usage id.
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

// === Part: create ===

/// `Part::create` rounds the purchase date via round_time().
#[tokio::test]
async fn part_create_sets_purchase_date() -> TbResult<()> {
    let mut store = MockStore::new();
    let sess = mock_session(UserId::from(99));
    let purchase = sample_purchase_date();
    // The store echoes back the purchase date the domain passes, so the
    // assertion below verifies the domain rounded it (not the store).
    store.expect_part_create().returning(
        |what, _name, _vendor, _model, p, _source, _usage, _owner, _shop| {
            Ok(Part {
                purchase: p,
                ..part_row(PartId::from(1), UserId::from(99), what)
            })
        },
    );
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

/// `Part::create` sets source.
#[tokio::test]
async fn part_create_with_source() -> TbResult<()> {
    let mut store = MockStore::new();
    let sess = mock_session(UserId::from(99));
    // The store echoes back the source the domain passes.
    store.expect_part_create().returning(
        |what, _name, _vendor, _model, _p, source, _usage, _owner, _shop| {
            Ok(Part {
                source,
                ..part_row(PartId::from(1), UserId::from(99), what)
            })
        },
    );
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

// === Part: by_source ===

/// `Part::by_source` returns the part carrying the given Strava source.
#[tokio::test]
async fn part_by_source_returns_part_with_source() -> TbResult<()> {
    let created = Part {
        source: Some("b12345".to_string()),
        ..part_row(PartId::from(1), test_user(), CHAIN)
    };
    let created_check = created.clone();
    let mut store = MockStore::new();
    store.expect_partid_get_by_source().returning(|src| {
        Ok(if src == "b12345" {
            Some(PartId::from(1))
        } else {
            None
        })
    });
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(created.clone()));

    let found = Part::by_source("b12345", &mut store).await?;
    assert_eq!(found, Some(created_check));
    Ok(())
}

/// `Part::by_source` returns None when no part carries the source.
#[tokio::test]
async fn part_by_source_returns_none_when_missing() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_partid_get_by_source().returning(|_| Ok(None));

    let found = Part::by_source("b-no-such-source", &mut store).await?;
    assert_eq!(found, None);
    Ok(())
}

// === Part: categories (the static PARTTYPES table) ===

/// `Part::categories` returns main part types only.
#[tokio::test]
async fn part_categories_returns_main_types() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_part_get_all_for_userid().returning(move |_| {
        Ok(vec![
            part_row(PartId::from(1), test_user(), BIKE),
            part_row(PartId::from(4), test_user(), CHAIN),
        ])
    });

    let categories = Part::categories(&test_session(), &mut store).await?;
    // Type 1 (Bike) is a main type, Type 4 (chain) is a subtype - only main
    // types returned.
    assert_eq!(categories.len(), 1);
    assert!(categories.contains(&BIKE));
    assert!(!categories.contains(&CHAIN));
    Ok(())
}

/// `Part::categories` returns empty when no parts.
#[tokio::test]
async fn part_categories_empty_for_no_parts() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_part_get_all_for_userid()
        .returning(|_| Ok(vec![]));

    let categories = Part::categories(&test_session(), &mut store).await?;
    assert!(categories.is_empty());
    Ok(())
}

/// `Part::categories` filters to main types only - subtypes are excluded.
#[tokio::test]
async fn part_categories_filters_subtypes() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_part_get_all_for_userid().returning(move |_| {
        Ok(vec![
            part_row(PartId::from(1), test_user(), BIKE),
            part_row(PartId::from(2), test_user(), FRONT_WHEEL),
        ])
    });

    // Type 1 (Bike) is main (hooks=[]), Type 2 (front wheel) is a subtype
    // (hooks=[1]).
    let categories = Part::categories(&test_session(), &mut store).await?;
    assert_eq!(categories.len(), 1);
    assert!(categories.contains(&BIKE));
    Ok(())
}

// === PartId: change ===

/// `PartId::change` updates part fields and returns the updated part.
#[tokio::test]
async fn part_change_updates_fields() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user(), BIKE)));
    store.expect_part_update().returning(Ok);

    let updated = PartId::from(1)
        .change(
            "New Name".to_string(),
            "New Vendor".to_string(),
            "New Model".to_string(),
            sample_purchase_date(),
            &test_session(),
            &mut store,
        )
        .await?;

    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.vendor, "New Vendor");
    assert_eq!(updated.model, "New Model");
    Ok(())
}

/// `PartId::change` returns forbidden for a non-owner session.
#[tokio::test]
async fn part_change_rejects_non_owner() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user(), BIKE)));

    let other_session = mock_session(UserId::from(98));
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

// === PartId: delete (access check) ===

/// `PartId::delete` returns forbidden for a non-owner session.
#[tokio::test]
async fn part_delete_rejects_non_owner() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user(), BIKE)));

    let other_session = mock_session(UserId::from(98));
    let result = PartId::from(1).delete(&other_session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}
