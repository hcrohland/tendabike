//! The `exec` dispatch over a canned [`MockStore`] (issue #638).
//!
//! Each test runs the real `exec` dispatch — the `Summary`/tombstone/guard
//! computation the handlers rely on — over a [`MockStore`] whose methods
//! return canned rows. The domain logic (guards, ownership checks, the
//! `Summary` build) runs for real; only the store I/O is canned. The five
//! dispatch cases that need a stateful store (attach/detach the stored row,
//! default-gear recalculation, subscription auto-approve, and the two
//! onboarding idempotency guards) live in the sqlx seam suite
//! (`sqlx/tests/store_seam.rs`), where the Postgres behavior is the source of
//! truth.

use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::{
    MockStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    mock_session, only_tombstone,
    part_type_ids::CHAIN,
    sole_live,
};
use crate::traits::{ActivityStore, PartNoteStore, PartStore};
use crate::{
    ActTypeId, Activity, ActivityId, Error, MAX_TIME, Part, PartId, PartNote, PartNoteId, Service,
    ServiceId, ServicePlan, ServicePlanId, Shop, ShopId, ShopSubscription, SubscriptionId,
    SubscriptionStatus, Summary, Usage, UsageId, UserId,
};
use time::macros::datetime;

/// The `Summary` of a write's outcome: every write under test reports
/// its `Summary` (the descend's `DescendReport` is asserted explicitly
/// where it is the point).
fn outcome_summary(outcome: WriteOutcome) -> Summary {
    match outcome {
        WriteOutcome::Summary(summary) => summary,
        WriteOutcome::Descend { summary, .. } => summary,
    }
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner).
fn part_row(id: PartId, owner: UserId) -> Part {
    Part {
        id,
        owner,
        what: CHAIN,
        name: "Spare Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-M510".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    }
}

/// A canned `Activity` row with no gear, so the register path is a no-op and
/// the dispatch's `Summary`/tombstone computation is what the test exercises.
fn activity_row(id: ActivityId, name: &str) -> Activity {
    Activity {
        id,
        user_id: test_user(),
        what: ActTypeId::from(1),
        name: name.to_string(),
        start: datetime!(2024-06-01 10:00 UTC),
        duration: 3600,
        time: Some(3500),
        distance: Some(50000),
        climb: Some(500),
        descend: None,
        energy: Some(1000),
        gear: None,
        device_name: None,
        external_id: None,
    }
}

/// A canned text `PartNote` row.
fn note_row(id: PartNoteId, part: PartId, name: &str) -> PartNote {
    PartNote {
        id,
        part,
        name: name.to_string(),
        mime: None,
        filename: None,
        size: None,
        created: sample_purchase_date(),
    }
}

/// A canned `Service` row.
fn service_row(id: ServiceId, part: PartId, name: &str) -> Service {
    Service {
        id,
        part_id: part,
        time: datetime!(2024-06-15 10:00 UTC),
        redone: MAX_TIME,
        name: name.to_string(),
        notes: "Old chain".to_string(),
        usage: UsageId::new(),
        successor: None,
        plans: vec![],
    }
}

/// A canned `Shop` row.
fn shop_row(id: ShopId, owner: UserId, name: &str, auto_approve: bool) -> Shop {
    Shop {
        id,
        owner,
        name: name.to_string(),
        description: None,
        auto_approve,
        created_at: sample_purchase_date(),
    }
}

/// A canned `ShopSubscription` row.
fn sub_row(
    id: SubscriptionId,
    shop: ShopId,
    user: UserId,
    status: SubscriptionStatus,
) -> ShopSubscription {
    ShopSubscription {
        id,
        shop_id: shop,
        user_id: user,
        status,
        message: None,
        response_message: None,
        created_at: sample_purchase_date(),
        updated_at: sample_purchase_date(),
    }
}

// --- Part ---

#[tokio::test]
async fn part_create_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let created = Part {
        id: PartId::from(100),
        owner: test_user(),
        what: CHAIN,
        name: "New Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-HG62".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    };
    store
        .expect_part_create()
        .returning(move |_, _, _, _, _, _, _, _, _| Ok(created.clone()));

    let summary = outcome_summary(
        exec(
            ApiWrite::PartCreate {
                name: "New Chain".to_string(),
                vendor: "Shimano".to_string(),
                model: "CN-HG62".to_string(),
                what: CHAIN,
                purchase: sample_purchase_date(),
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.len(), 1);
    let part = sole_live(&summary.parts);
    assert_eq!(part.name, "New Chain");
    assert_eq!(part.owner, test_user());
    assert_eq!(part.what, CHAIN);
    Ok(())
}

#[tokio::test]
async fn part_change_dispatches() -> TbResult<()> {
    let id = PartId::from(13);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(id, test_user())));
    let renamed = Part {
        name: "Renamed".to_string(),
        ..part_row(id, test_user())
    };
    store
        .expect_part_update()
        .returning(move |_| Ok(renamed.clone()));

    let summary = outcome_summary(
        exec(
            ApiWrite::PartChange {
                id,
                name: "Renamed".to_string(),
                vendor: "New Vendor".to_string(),
                model: "New Model".to_string(),
                purchase: sample_purchase_date(),
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.get(&id).unwrap().name, "Renamed");
    Ok(())
}

#[tokio::test]
async fn part_delete_reports_tombstone() -> TbResult<()> {
    let id = PartId::from(13);
    let mut store = MockStore::new();
    // The part is read by the op's ownership check, then read again by the
    // test's post-delete assert — the second read must be a not-found.
    let counter = Arc::new(Mutex::new(0));
    let counter_calls = counter.clone();
    store.expect_partid_get_part().returning(move |_| {
        let n = *counter_calls.lock().unwrap();
        *counter_calls.lock().unwrap() += 1;
        if n == 0 {
            Ok(part_row(id, test_user()))
        } else {
            Err(Error::NotFound("part".into()))
        }
    });
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_by_part().returning(|_| Ok(vec![]));
    store.expect_part_delete().returning(move |_| Ok(id));

    let summary =
        outcome_summary(exec(ApiWrite::PartDelete { id }, &test_session(), &mut store).await?);
    assert_eq!(summary.parts, only_tombstone(id));
    assert!(store.partid_get_part(id).await.is_err());
    Ok(())
}

#[tokio::test]
async fn part_delete_missing_is_not_found() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(|_| Err(Error::NotFound("part".into())));

    let err = exec(
        ApiWrite::PartDelete {
            id: PartId::from(9999),
        },
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// --- Activity ---

#[tokio::test]
async fn activity_update_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let stored = activity_row(ActivityId::new(1), "Morning Ride");
    let stored_read = stored.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(stored_read.clone())));
    store.expect_activity_update().returning(Ok);

    let mut act = stored.clone();
    let id = act.id;
    act.name = "Renamed Ride".to_string();
    let summary = outcome_summary(
        exec(
            ApiWrite::ActivityUpdate { id, activity: act },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.activities.get(&id).unwrap().name, "Renamed Ride");
    Ok(())
}

#[tokio::test]
async fn activity_update_id_mismatch_is_bad_request() -> TbResult<()> {
    let mut store = MockStore::new();
    let stored = activity_row(ActivityId::new(1), "Morning Ride");
    let stored_read = stored.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(stored_read.clone())));

    let act = stored;
    // A path id different from the body's: the handler's guard.
    let err = exec(
        ApiWrite::ActivityUpdate {
            id: ActivityId::new(2),
            activity: act,
        },
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

#[tokio::test]
async fn activity_delete_reports_tombstone() -> TbResult<()> {
    let id = ActivityId::new(3);
    let mut store = MockStore::new();
    // The activity is read by the delete, then read again by the test's
    // post-delete assert — the second read must be gone.
    let counter = Arc::new(Mutex::new(0));
    let counter_calls = counter.clone();
    store.expect_activity_read_by_id().returning(move |_| {
        let n = *counter_calls.lock().unwrap();
        *counter_calls.lock().unwrap() += 1;
        if n == 0 {
            Ok(Some(activity_row(id, "Ride")))
        } else {
            Ok(None)
        }
    });
    store.expect_activity_delete().returning(move |_| Ok(1));

    let summary =
        outcome_summary(exec(ApiWrite::ActivityDelete { id }, &test_session(), &mut store).await?);
    assert_eq!(summary.activities, only_tombstone(id));
    assert!(store.activity_read_by_id(id).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn activity_delete_missing_is_not_found() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_activity_read_by_id().returning(|_| Ok(None));

    let err = exec(
        ApiWrite::ActivityDelete {
            id: ActivityId::new(999_999),
        },
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

#[tokio::test]
async fn activity_descend_updates_matched_activity() -> TbResult<()> {
    let mut store = MockStore::new();
    let matched = activity_row(ActivityId::new(1), "Morning Ride");
    let matched_lookup = matched.clone();
    store
        .expect_get_by_user_and_time()
        .returning(move |_, _| Ok(matched_lookup.clone()));
    let matched_read = matched.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(matched_read.clone())));
    store.expect_activity_update().returning(Ok);

    let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n";
    let outcome = exec(
        ApiWrite::ActivityDescend {
            data: csv.to_string(),
        },
        &test_session(),
        &mut store,
    )
    .await?;
    let WriteOutcome::Descend { summary, report } = outcome else {
        panic!("the descend write carries its match report");
    };
    assert_eq!(
        summary.activities.get(&ActivityId::new(1)).unwrap().descend,
        Some(900)
    );
    // The dispatch carries the match report (the handler's response
    // body), not just the Summary.
    assert_eq!(
        report,
        DescendReport {
            good: vec!["Morning Ride at 2023-05-18 22:13:20".to_string()],
            bad: vec![],
        }
    );
    Ok(())
}

// --- PartNote ---

#[tokio::test]
async fn partnote_create_text_and_delete_dispatch() -> TbResult<()> {
    let part = PartId::from(13);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    let note = note_row(PartNoteId::from(1), part, "Check the tension");
    let note_created = note.clone();
    store
        .expect_partnote_create_text()
        .returning(move |_, _, _| Ok(note_created.clone()));
    // The note is read by the delete's access check, then read again by the
    // test's post-delete assert — the second read must be a not-found.
    let counter = Arc::new(Mutex::new(0));
    let counter_calls = counter.clone();
    let note_get = note.clone();
    store.expect_partnote_get().returning(move |_| {
        let n = *counter_calls.lock().unwrap();
        *counter_calls.lock().unwrap() += 1;
        if n == 0 {
            Ok(note_get.clone())
        } else {
            Err(Error::NotFound("note".into()))
        }
    });
    let note_id = note.id;
    store
        .expect_partnote_delete()
        .returning(move |_| Ok(note_id));

    let summary = outcome_summary(
        exec(
            ApiWrite::PartNoteCreateText {
                part,
                name: "Check the tension".to_string(),
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.part_notes.len(), 1);
    let note = sole_live(&summary.part_notes);
    assert_eq!(note.name, "Check the tension");
    assert_eq!(note.part, part);
    assert!(!note.has_file());

    let summary = outcome_summary(
        exec(
            ApiWrite::PartNoteDelete { id: note.id },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.part_notes, only_tombstone(note.id));
    assert!(store.partnote_get(note.id).await.is_err());
    Ok(())
}

#[tokio::test]
async fn partnote_blank_name_is_bad_request() -> TbResult<()> {
    let mut store = MockStore::new();

    let err = exec(
        ApiWrite::PartNoteCreateText {
            part: PartId::from(13),
            name: "   ".to_string(),
        },
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

// --- Service ---

#[tokio::test]
async fn service_create_and_delete_dispatch() -> TbResult<()> {
    let part = PartId::from(13);
    let time = datetime!(2024-06-15 10:00 UTC);
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    // A loose spare chain: not main, no attachments, no services.
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(1));
    let service = service_row(ServiceId::new(), part, "Chain Service");
    let service_created = service.clone();
    store
        .expect_service_create()
        .returning(move |_| Ok(service_created.clone()));
    let service_get = service.clone();
    store
        .expect_service_get()
        .returning(move |_| Ok(service_get.clone()));
    store
        .expect_usage_delete()
        .returning(|_| Ok(Usage::default()));
    store.expect_service_delete().returning(|_| Ok(1));

    let summary = outcome_summary(
        exec(
            ApiWrite::ServiceCreate {
                part,
                time,
                name: "Chain Service".to_string(),
                notes: "Old chain".to_string(),
                plans: vec![],
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.services.len(), 1);
    assert_eq!(summary.usages.len(), 1);
    let service = sole_live(&summary.services);
    assert_eq!(service.name, "Chain Service");

    let summary = outcome_summary(
        exec(
            ApiWrite::ServiceDelete { id: service.id },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.services, only_tombstone(service.id));
    Ok(())
}

#[tokio::test]
async fn service_create_on_foreign_part_is_forbidden() -> TbResult<()> {
    let mut store = MockStore::new();
    // Part 1 is owned by user 1; user 2 (no shop) may not service it.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), test_user())));

    let session = mock_session(UserId::from(2));
    let err = exec(
        ApiWrite::ServiceCreate {
            part: PartId::from(1),
            time: datetime!(2024-06-15 10:00 UTC),
            name: "Service".to_string(),
            notes: "".to_string(),
            plans: vec![],
        },
        &session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

// --- ServicePlan ---

#[tokio::test]
async fn serviceplan_create_and_delete_dispatch() -> TbResult<()> {
    let part = PartId::from(13);
    let plan = ServicePlan {
        id: ServicePlanId::from(uuid::Uuid::now_v7()),
        part: Some(part),
        what: CHAIN,
        hook: None,
        name: "Chain Every 1000km".to_string(),
        days: None,
        hours: None,
        km: Some(1000),
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };
    let mut store = MockStore::new();
    let plan_created = plan.clone();
    store
        .expect_serviceplan_create()
        .returning(move |_| Ok(plan_created.clone()));
    let plan_get = plan.clone();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(plan_get.clone()));
    // The delete's ownership check and the unlink scan (no services here).
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    store
        .expect_part_get_all_for_userid()
        .returning(move |_| Ok(vec![part_row(part, test_user())]));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_serviceplan_delete().returning(|_| Ok(1));

    let summary = outcome_summary(
        exec(
            ApiWrite::ServicePlanCreate { plan },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.plans.len(), 1);
    // The create assigns a fresh server id (the caller's is discarded).
    let plan_id = sole_live(&summary.plans).id;

    let summary = outcome_summary(
        exec(
            ApiWrite::ServicePlanDelete { id: plan_id },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.plans, only_tombstone(plan_id));
    Ok(())
}

// --- Shop ---

#[tokio::test]
async fn shop_create_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let shop = shop_row(ShopId::from(99), test_user(), "Workshop", true);
    store
        .expect_shop_create()
        .returning(move |_, _, _, _| Ok(shop.clone()));

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.shops.len(), 1);
    let shop = sole_live(&summary.shops);
    assert_eq!(shop.name, "Workshop");
    assert_eq!(shop.owner, test_user());
    Ok(())
}

#[tokio::test]
async fn shop_register_and_unregister_part_dispatch() -> TbResult<()> {
    let part = PartId::from(13); // loose spare, owned by user 1
    let shop_id = ShopId::from(99);
    let mut store = MockStore::new();

    // The shop create.
    let shop = shop_row(shop_id, test_user(), "Workshop", true);
    let shop_created = shop.clone();
    store
        .expect_shop_create()
        .returning(move |_, _, _, _| Ok(shop_created.clone()));

    // The route's checkuser requires an (owner) subscription, so the test
    // creates one first — auto-approve activates it.
    let shop_get = shop.clone();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_get.clone()));
    store
        .expect_subscription_find_pending()
        .returning(|_, _| Ok(None));
    store
        .expect_subscription_find_active()
        .returning(|_, _| Ok(None));
    let pending = sub_row(
        SubscriptionId::from(1),
        shop_id,
        test_user(),
        SubscriptionStatus::Pending,
    );
    store
        .expect_subscription_create()
        .returning(move |_, _, _| Ok(pending.clone()));
    let approved = sub_row(
        SubscriptionId::from(1),
        shop_id,
        test_user(),
        SubscriptionStatus::Active,
    );
    store
        .expect_subscription_approve()
        .returning(move |_, _, _| Ok(approved.clone()));

    // The register/unregister: the subscription is active, the part is a
    // loose spare (not attached, no subparts).
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            test_user(),
            SubscriptionStatus::Active,
        )])
    });
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(|_, _, _| Ok(vec![]));
    let registered = Part {
        shop: Some(shop_id),
        ..part_row(part, test_user())
    };
    store
        .expect_parts_register_shop()
        .returning(move |_, _| Ok(vec![registered.clone()]));
    store
        .expect_parts_unregister_shop()
        .returning(move |_| Ok(vec![part_row(part, test_user())]));

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    let shop_id = sole_live(&summary.shops).id;

    exec(
        ApiWrite::ShopSubscriptionCreate {
            shop: shop_id,
            message: None,
        },
        &test_session(),
        &mut store,
    )
    .await?;

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopRegisterPart {
                shop: shop_id,
                part,
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.get(&part).unwrap().shop, Some(shop_id));

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopUnregisterPart {
                shop: shop_id,
                part,
            },
            &test_session(),
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.get(&part).unwrap().shop, None);
    Ok(())
}
