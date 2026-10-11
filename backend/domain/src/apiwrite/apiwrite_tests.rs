use super::*;
use crate::test_support::{
    MemStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    only_tombstone,
    part_type_ids::{BIKE, CHAIN},
    sole_live,
};
use crate::traits::{
    ActivityStore, AttachmentStore, PartNoteStore, PartStore, ShopStore, UserStore,
};
use crate::{
    ActTypeId, Activity, ActivityId, Error, MAX_TIME, OnboardingStatus, Part, PartId, PartTypeId,
    ServicePlan, ServicePlanId, SubscriptionStatus, Summary, UserId,
};
use time::macros::datetime;

/// A fresh hook on a bike that a chain can be attached to (the first hook
/// of the chain part type, like the in-memory attachment suite does).
fn chain_hook() -> PartTypeId {
    CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN)
}

/// The `Summary` of a write's outcome: every write under test reports
/// its `Summary` (the descend's `DescendReport` is asserted explicitly
/// where it is the point).
fn outcome_summary(outcome: WriteOutcome) -> Summary {
    match outcome {
        WriteOutcome::Summary(summary) => summary,
        WriteOutcome::Descend { summary, .. } => summary,
    }
}

// --- Part ---

#[tokio::test]
async fn part_create_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary = outcome_summary(
        exec(
            ApiWrite::PartCreate {
                name: "New Chain".to_string(),
                vendor: "Shimano".to_string(),
                model: "CN-HG62".to_string(),
                what: CHAIN,
                purchase: sample_purchase_date(),
            },
            &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let id = PartId::from(13); // "Spare Chain 1" — loose, owned by user 1
    let summary = outcome_summary(
        exec(
            ApiWrite::PartChange {
                id,
                name: "Renamed".to_string(),
                vendor: "New Vendor".to_string(),
                model: "New Model".to_string(),
                purchase: sample_purchase_date(),
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.get(&id).unwrap().name, "Renamed");
    Ok(())
}

#[tokio::test]
async fn part_delete_reports_tombstone() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let id = PartId::from(13);
    let summary =
        outcome_summary(exec(ApiWrite::PartDelete { id }, &mut session, &mut store).await?);
    assert_eq!(summary.parts, only_tombstone(id));
    assert!(store.partid_get_part(id).await.is_err());
    Ok(())
}

#[tokio::test]
async fn part_delete_missing_is_not_found() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let err = exec(
        ApiWrite::PartDelete {
            id: PartId::from(9999),
        },
        &mut session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// --- Attachment ---

#[tokio::test]
async fn attach_and_detach_dispatch() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let bike = Part::create(
        "Bike".to_string(),
        "V".to_string(),
        "M".to_string(),
        BIKE,
        None,
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await?;
    let chain = Part::create(
        "Chain".to_string(),
        "Shimano".to_string(),
        "CN-M510".to_string(),
        CHAIN,
        None,
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await?;
    let time = datetime!(2024-01-01 00:00 UTC);

    let summary = outcome_summary(
        exec(
            ApiWrite::AttachmentAttach {
                part: chain.id,
                time,
                gear: bike.id,
                hook: chain_hook(),
                all: false,
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert!(!summary.parts.is_empty());
    let att = store
        .attachment_get_by_part_and_time(chain.id, time)
        .await?
        .expect("the chain is attached");
    assert_eq!(att.gear, bike.id);
    assert_eq!(att.detached, MAX_TIME);

    let _ = exec(
        ApiWrite::AttachmentDetach {
            part: chain.id,
            time,
            all: false,
        },
        &mut session,
        &mut store,
    )
    .await?;
    assert!(
        store
            .attachment_get_by_part_and_time(chain.id, time)
            .await?
            .is_none()
    );
    Ok(())
}

// --- Activity ---

#[tokio::test]
async fn activity_update_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let mut act = store
        .activity_read_by_id(ActivityId::new(1))
        .await?
        .expect("the fixture activity");
    let id = act.id;
    act.name = "Renamed Ride".to_string();
    let summary = outcome_summary(
        exec(
            ApiWrite::ActivityUpdate { id, activity: act },
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.activities.get(&id).unwrap().name, "Renamed Ride");
    Ok(())
}

#[tokio::test]
async fn activity_update_id_mismatch_is_bad_request() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let act = store
        .activity_read_by_id(ActivityId::new(1))
        .await?
        .expect("the fixture activity");
    // A path id different from the body's: the handler's guard.
    let err = exec(
        ApiWrite::ActivityUpdate {
            id: ActivityId::new(2),
            activity: act,
        },
        &mut session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::BadRequest(_)));
    Ok(())
}

#[tokio::test]
async fn activity_delete_reports_tombstone() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let id = ActivityId::new(3);
    let summary =
        outcome_summary(exec(ApiWrite::ActivityDelete { id }, &mut session, &mut store).await?);
    assert_eq!(summary.activities, only_tombstone(id));
    assert!(store.activity_read_by_id(id).await?.is_none());
    Ok(())
}

#[tokio::test]
async fn activity_delete_missing_is_not_found() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let err = exec(
        ApiWrite::ActivityDelete {
            id: ActivityId::new(999_999),
        },
        &mut session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

#[tokio::test]
async fn activity_descend_updates_matched_activity() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    // A row matching fixture activity 1 (start 2023-05-18 22:13:20Z,
    // matched by local minute) with a descend value.
    let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n";
    let outcome = exec(
        ApiWrite::ActivityDescend {
            data: csv.to_string(),
        },
        &mut session,
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

#[tokio::test]
async fn activity_default_gear_assigns_null_gear_activities() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let act = Activity {
        id: ActivityId::new(100),
        user_id: test_user(),
        what: ActTypeId::from(1),
        name: "No Gear".to_string(),
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
    };
    store.activity_create(act.clone()).await?;

    let summary = outcome_summary(
        exec(
            ApiWrite::ActivityDefaultGear {
                gear: PartId::from(1), // "Main Bike" in the fixture
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    let stored = store
        .activity_read_by_id(act.id)
        .await?
        .expect("the activity still exists");
    assert_eq!(stored.gear, Some(PartId::from(1)));
    assert!(
        summary
            .activities
            .live()
            .into_iter()
            .any(|a| a.id == act.id)
    );
    Ok(())
}

// --- PartNote ---

#[tokio::test]
async fn partnote_create_text_and_delete_dispatch() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let part = PartId::from(13);
    let summary = outcome_summary(
        exec(
            ApiWrite::PartNoteCreateText {
                part,
                name: "Check the tension".to_string(),
            },
            &mut session,
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
            &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let err = exec(
        ApiWrite::PartNoteCreateText {
            part: PartId::from(13),
            name: "   ".to_string(),
        },
        &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let time = datetime!(2024-06-15 10:00 UTC);
    let summary = outcome_summary(
        exec(
            ApiWrite::ServiceCreate {
                part: PartId::from(13),
                time,
                name: "Chain Service".to_string(),
                notes: "Old chain".to_string(),
                plans: vec![],
            },
            &mut session,
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
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.services, only_tombstone(service.id));
    Ok(())
}

#[tokio::test]
async fn service_create_on_foreign_part_is_forbidden() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    // Part 1 is owned by user 1; user 2 may not service it.
    let mut session = crate::test_support::TestSession::new(UserId::from(2));
    let err = exec(
        ApiWrite::ServiceCreate {
            part: PartId::from(1),
            time: datetime!(2024-06-15 10:00 UTC),
            name: "Service".to_string(),
            notes: "".to_string(),
            plans: vec![],
        },
        &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let plan = ServicePlan {
        id: ServicePlanId::from(uuid::Uuid::now_v7()),
        part: Some(PartId::from(13)),
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
    let summary = outcome_summary(
        exec(
            ApiWrite::ServicePlanCreate { plan },
            &mut session,
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
            &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary = outcome_summary(
        exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &mut session,
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
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary = outcome_summary(
        exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    let shop_id = sole_live(&summary.shops).id;
    // The route's checkuser requires an (owner) subscription, so create
    // one first — auto-approve activates it.
    exec(
        ApiWrite::ShopSubscriptionCreate {
            shop: shop_id,
            message: None,
        },
        &mut session,
        &mut store,
    )
    .await?;
    let part = PartId::from(13); // loose spare, owned by user 1

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopRegisterPart {
                shop: shop_id,
                part,
            },
            &mut session,
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
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary.parts.get(&part).unwrap().shop, None);
    Ok(())
}

#[tokio::test]
async fn shop_subscription_create_and_cancel_dispatch() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary = outcome_summary(
        exec(
            ApiWrite::ShopCreate {
                name: "Workshop".to_string(),
                description: None,
                auto_approve: true,
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    let shop_id = sole_live(&summary.shops).id;

    // A subscription is not a Summary kind: the write reports an empty
    // Summary, the side effect lands in the store.
    let summary = outcome_summary(
        exec(
            ApiWrite::ShopSubscriptionCreate {
                shop: shop_id,
                message: Some("Please approve".to_string()),
            },
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary, Summary::default());
    let sub = store
        .subscription_find_active(shop_id, test_user())
        .await?
        .expect("auto-approve activates the subscription");
    assert_eq!(sub.status, SubscriptionStatus::Active);

    let summary = outcome_summary(
        exec(
            ApiWrite::ShopSubscriptionCancel { id: sub.id },
            &mut session,
            &mut store,
        )
        .await?,
    );
    assert_eq!(summary, Summary::default());
    assert!(store.subscription_get(sub.id).await.is_err());
    Ok(())
}

// --- User (onboarding) ---

#[tokio::test]
async fn user_onboarding_sync_dispatch() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary = outcome_summary(
        exec(
            ApiWrite::UserOnboardingSync { time: 0 },
            &mut session,
            &mut store,
        )
        .await?,
    );
    // A User is not in any Summary.
    assert_eq!(summary, Summary::default());
    let user = UserStore::get(&mut store, UserId::from(1)).await?;
    assert_eq!(user.onboarding_status, OnboardingStatus::Completed);

    // The handler's guard: a second sync is rejected with the same message.
    let err = exec(
        ApiWrite::UserOnboardingSync { time: 0 },
        &mut session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        &err,
        Error::BadRequest(msg) if msg == "Initial sync already triggered"
    ));
    Ok(())
}

#[tokio::test]
async fn user_onboarding_postpone_dispatch() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let mut session = test_session();
    let summary =
        outcome_summary(exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store).await?);
    assert_eq!(summary, Summary::default());
    let user = UserStore::get(&mut store, UserId::from(1)).await?;
    assert_eq!(
        user.onboarding_status,
        OnboardingStatus::InitialSyncPostponed
    );

    // The handler's guard: not pending anymore.
    let err = exec(ApiWrite::UserOnboardingPostpone, &mut session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(
        &err,
        Error::BadRequest(msg)
            if msg == "Initial sync already completed or postponed"
    ));
    Ok(())
}
