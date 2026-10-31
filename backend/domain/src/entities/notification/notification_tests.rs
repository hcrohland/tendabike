use std::str::FromStr;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::test_support::{MockStore, fixtures::test_user};
use crate::{
    Notification, NotificationId, NotificationKind, NotificationLifecycle, NotificationState,
    NotificationSubject, PartId, SubscriptionId, Summary, UserId,
};

/// A fixed v7-style UUID for test notifications.
fn test_notification_id() -> NotificationId {
    NotificationId::from(Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000001").unwrap())
}

/// A fixed timestamp for test notifications.
fn test_created_at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}

/// A persistent notification (subscription kind) in the `new` state.
fn persistent_notification() -> Notification {
    Notification {
        id: test_notification_id(),
        recipient: test_user(),
        lifecycle: NotificationLifecycle::Persistent,
        kind: NotificationKind::SubRequest,
        subject: NotificationSubject::Subscription(SubscriptionId::from(1)),
        message: Some("Please approve".to_string()),
        state: NotificationState::New,
        created_at: test_created_at(),
    }
}

/// A transient notification (part kind) in the `new` state.
fn transient_notification() -> Notification {
    Notification {
        id: test_notification_id(),
        recipient: test_user(),
        lifecycle: NotificationLifecycle::Transient,
        kind: NotificationKind::PartRegistered,
        subject: NotificationSubject::Part(PartId::from(1)),
        message: None,
        state: NotificationState::New,
        created_at: test_created_at(),
    }
}

// --- Tier A: pure value-logic tests ---

#[test]
fn notification_id_roundtrips_through_uuid() {
    let id = test_notification_id();
    let uuid: Uuid = id.into();
    let back: NotificationId = uuid.into();
    assert_eq!(id, back);
}

#[test]
fn notification_lifecycle_serde_is_lowercase() {
    let persistent = serde_json::to_string(&NotificationLifecycle::Persistent).unwrap();
    assert_eq!(persistent, "\"persistent\"");
    let transient = serde_json::to_string(&NotificationLifecycle::Transient).unwrap();
    assert_eq!(transient, "\"transient\"");
}

#[test]
fn notification_kind_serde_is_snake_case() {
    let kind = serde_json::to_string(&NotificationKind::SubRequest).unwrap();
    assert_eq!(kind, "\"sub_request\"");
    let kind = serde_json::to_string(&NotificationKind::PartRegistered).unwrap();
    assert_eq!(kind, "\"part_registered\"");
}

#[test]
fn notification_state_serde_is_lowercase() {
    let new = serde_json::to_string(&NotificationState::New).unwrap();
    assert_eq!(new, "\"new\"");
    let seen = serde_json::to_string(&NotificationState::Seen).unwrap();
    assert_eq!(seen, "\"seen\"");
}

#[test]
fn notification_subject_subscription_serde() {
    let subject = NotificationSubject::Subscription(SubscriptionId::from(42));
    let json = serde_json::to_string(&subject).unwrap();
    assert_eq!(json, r#"{"type":"subscription","id":42}"#);
    let back: NotificationSubject = serde_json::from_str(&json).unwrap();
    assert_eq!(subject, back);
}

#[test]
fn notification_subject_part_serde() {
    let subject = NotificationSubject::Part(PartId::from(7));
    let json = serde_json::to_string(&subject).unwrap();
    assert_eq!(json, r#"{"type":"part","id":7}"#);
    let back: NotificationSubject = serde_json::from_str(&json).unwrap();
    assert_eq!(subject, back);
}

// --- Tier B: single-entity CRUD over MockStore ---

#[tokio::test]
async fn notification_read_persistent_marks_seen_and_upserts() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let mut notification = persistent_notification();
    // The store returns the notification with state = Seen (the store does the lifecycle work).
    notification.state = NotificationState::Seen;
    store
        .expect_notification_read()
        .withf(move |id, recipient| *id == test_notification_id() && *recipient == test_user())
        .times(1)
        .returning(move |_, _| Ok(Some(notification.clone())));

    let summary = test_notification_id().read(test_user(), &mut store).await?;
    let read = summary.notifications.get(&test_notification_id());
    assert!(read.is_some(), "persistent notification should be upserted");
    let read = read.unwrap();
    assert_eq!(read.state, NotificationState::Seen);
    assert_eq!(read.lifecycle, NotificationLifecycle::Persistent);
    assert!(!summary.notifications.is_tombstoned(&test_notification_id()));
    Ok(())
}

#[tokio::test]
async fn notification_read_transient_tombstones() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let notification = transient_notification();
    store
        .expect_notification_read()
        .withf(move |id, recipient| *id == test_notification_id() && *recipient == test_user())
        .times(1)
        .returning(move |_, _| Ok(Some(notification.clone())));

    let summary = test_notification_id().read(test_user(), &mut store).await?;
    assert!(
        summary.notifications.is_tombstoned(&test_notification_id()),
        "transient notification should be tombstoned"
    );
    assert!(
        summary.notifications.get(&test_notification_id()).is_none(),
        "transient notification should not be live"
    );
    Ok(())
}

#[tokio::test]
async fn notification_read_non_recipient_returns_not_found() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    // The store returns None for a non-recipient (the guard is in the DB query).
    store
        .expect_notification_read()
        .withf(|_id, recipient| *recipient != test_user())
        .times(1)
        .returning(|_, _| Ok(None));

    let result = test_notification_id()
        .read(UserId::from(999), &mut store)
        .await;
    assert!(
        matches!(result, Err(crate::Error::NotFound(_))),
        "non-recipient should get NotFound, got {result:?}"
    );
    Ok(())
}

#[tokio::test]
async fn notification_read_all_mixed_lifecycles() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let persistent = {
        let mut n = persistent_notification();
        n.state = NotificationState::Seen;
        n
    };
    let persistent_id = persistent.id;
    let transient = {
        let mut n = transient_notification();
        n.id =
            NotificationId::from(Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000005").unwrap());
        n
    };
    let transient_id = transient.id;
    store
        .expect_notification_read_all()
        .withf(move |uid| *uid == test_user())
        .times(1)
        .returning(move |_| Ok(vec![persistent.clone(), transient.clone()]));

    let summary = Notification::read_all(test_user(), &mut store).await?;
    // Persistent: upserted (now seen)
    assert!(summary.notifications.get(&persistent_id).is_some());
    assert_eq!(
        summary.notifications.get(&persistent_id).unwrap().state,
        NotificationState::Seen
    );
    // Transient: tombstoned
    assert!(summary.notifications.is_tombstoned(&transient_id));
    assert!(summary.notifications.get(&transient_id).is_none());
    Ok(())
}

#[tokio::test]
async fn notification_read_all_persistent_only() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let mut n1 = persistent_notification();
    n1.state = NotificationState::Seen;
    let n1_id = n1.id;
    let mut n2 = persistent_notification();
    n2.id = NotificationId::from(Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000002").unwrap());
    n2.state = NotificationState::Seen;
    let n2_id = n2.id;
    store
        .expect_notification_read_all()
        .withf(move |uid| *uid == test_user())
        .times(1)
        .returning(move |_| Ok(vec![n1.clone(), n2.clone()]));

    let summary = Notification::read_all(test_user(), &mut store).await?;
    assert_eq!(summary.notifications.len(), 2);
    assert!(summary.notifications.get(&n1_id).is_some());
    assert!(summary.notifications.get(&n2_id).is_some());
    assert!(!summary.notifications.is_tombstoned(&n1_id));
    assert!(!summary.notifications.is_tombstoned(&n2_id));
    Ok(())
}

#[tokio::test]
async fn notification_read_all_transient_only_tombstones_all() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let t1 = transient_notification();
    let t1_id = t1.id;
    let t2 = Notification {
        id: NotificationId::from(Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000003").unwrap()),
        ..transient_notification()
    };
    let t2_id = t2.id;
    store
        .expect_notification_read_all()
        .withf(move |uid| *uid == test_user())
        .times(1)
        .returning(move |_| Ok(vec![t1.clone(), t2.clone()]));

    let summary = Notification::read_all(test_user(), &mut store).await?;
    assert_eq!(summary.notifications.len(), 2);
    assert!(summary.notifications.is_tombstoned(&t1_id));
    assert!(summary.notifications.is_tombstoned(&t2_id));
    assert!(summary.notifications.get(&t1_id).is_none());
    assert!(summary.notifications.get(&t2_id).is_none());
    Ok(())
}

// --- Tier C: cross-entity integration (Summary hydration) ---

#[tokio::test]
async fn get_summary_includes_notifications() -> crate::TbResult<()> {
    let mut store = MockStore::new();
    let user = test_user();

    // Script all the store methods that get_summary calls.
    store.expect_user_get().returning(move |_| {
        Ok(crate::User {
            id: user,
            name: "Test User".to_string(),
            firstname: "Test".to_string(),
            avatar: None,
            is_admin: false,
            onboarding_status: crate::OnboardingStatus::Completed,
        })
    });
    store.expect_get_all().returning(|_| Ok(vec![]));
    store
        .expect_shops_get_all_for_user()
        .returning(|_| Ok(vec![]));
    store
        .expect_part_get_all_for_userid()
        .returning(|_| Ok(vec![]));
    // get_part_summary calls ServicePlan::for_user even with no parts
    store.expect_by_user().returning(|_| Ok(vec![]));
    store
        .expect_notification_all_for_user()
        .withf(move |uid| *uid == user)
        .returning(|_| {
            let n1 = persistent_notification();
            let mut n2 = transient_notification();
            n2.id = NotificationId::from(
                Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000006").unwrap(),
            );
            Ok(vec![n1, n2])
        });

    let summary = user.get_summary(None, None, &mut store).await?;
    assert_eq!(summary.notifications.len(), 2);
    assert!(summary.notifications.get(&test_notification_id()).is_some());
    Ok(())
}

#[tokio::test]
async fn notification_summary_merge_works() -> crate::TbResult<()> {
    let mut n1 = persistent_notification();
    n1.state = NotificationState::Seen;
    let mut n2 = persistent_notification();
    n2.id = NotificationId::from(Uuid::from_str("018f9c2e-4b3a-7000-8000-000000000004").unwrap());

    let mut s1 = Summary::default();
    s1.notifications.upsert(n1.clone());
    let mut s2 = Summary::default();
    s2.notifications.upsert(n2.clone());
    s2.notifications.tombstone(test_notification_id());

    s1 += s2;

    // n1 was tombstoned by s2
    assert!(s1.notifications.is_tombstoned(&n1.id));
    // n2 was upserted by s2
    assert!(s1.notifications.get(&n2.id).is_some());
    Ok(())
}

#[test]
fn summary_is_empty_with_notifications() {
    let mut summary = Summary::default();
    assert!(summary.is_empty());
    summary.notifications.upsert(persistent_notification());
    assert!(!summary.is_empty());
}
