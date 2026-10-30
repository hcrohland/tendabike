//! The `ApiRead` dispatch over a canned [`MockStore`] (issue #639).
//!
//! Each test runs the real `read` dispatch — the `ReadOutcome`/projections/
//! filters/access-checks the read handlers rely on — over a [`MockStore`]
//! whose methods return canned rows. The domain logic (the `UserState`
//! projection, the shop-scoped part filter, the `UserPublic` projection, the
//! pending-only subscription filter, the ownership/access checks) runs for
//! real; only the store I/O is canned. The write→read-back stored-value
//! cases (the shop catalog round-trips and the part-note file round-trip)
//! live in the sqlx seam suite (`sqlx/tests/store_seam.rs`), where the
//! Postgres behavior is the source of truth.

use std::collections::HashSet;

use super::*;
use crate::test_support::{
    MockStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    mock_session, mock_session_with_shop,
    part_type_ids::{BIKE, CHAIN},
};
use crate::{
    ActTypeId, Activity, ActivityId, Error, OnboardingStatus, Part, PartId, PartNote, PartNoteId,
    PartTypeId, Shop, ShopId, ShopSubscription, SubscriptionId, SubscriptionStatus, Usage, UsageId,
    User, UserId, UserPublic,
};
use time::macros::datetime;

// --- Canned rows: the minimal shape the domain logic needs ---

/// A canned `User` row: the test user ("Tenda"/"Bike"), not admin, onboarding pending.
fn user_row(id: UserId) -> User {
    User {
        id,
        name: "Bike".to_string(),
        firstname: "Tenda".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Pending,
    }
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner + type).
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

/// A canned `Activity` row.
fn activity_row(id: ActivityId, user: UserId, name: &str) -> Activity {
    Activity {
        id,
        user_id: user,
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

/// A canned `Usage` row carrying the requested id (so a part and its usages
/// stay distinct when the summary dedupes by id).
fn usage_row(id: UsageId) -> Usage {
    Usage {
        id,
        time: 0,
        distance: 0,
        climb: 0,
        descend: 0,
        energy: 0,
        count: 0,
    }
}

// --- User (the unified read: UserState = User + Summary) ---

/// `ApiRead::User` is the unified user read (spec #558 §3.5): the `UserState`
/// composite — the full `User` plus the `Summary` of everything the user owns
/// (the session here carries no shop). The projection is computed from the
/// canned store rows: the bare `User` record, the summary's parts and
/// activities, and the per-part usages.
#[tokio::test]
async fn user_read_returns_user_state() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    store.expect_user_get().returning(move |_| Ok(user_row(me)));
    store.expect_get_all().returning(move |_| {
        Ok(vec![
            activity_row(ActivityId::new(1), me, "Morning Ride"),
            activity_row(ActivityId::new(2), me, "Evening Ride"),
        ])
    });
    store
        .expect_shops_get_all_for_user()
        .returning(|_| Ok(vec![]));
    // Two parts, each with its own usage; no attachments/services/notes/plans.
    let bike = part_row(PartId::from(1), me, BIKE);
    let chain = part_row(PartId::from(4), me, CHAIN);
    store
        .expect_part_get_all_for_userid()
        .returning(move |_| Ok(vec![bike.clone(), chain.clone()]));
    store.expect_by_user().returning(|_| Ok(vec![]));
    store
        .expect_usage_get()
        .returning(|id| Ok(Some(usage_row(id))));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_by_part().returning(|_| Ok(vec![]));
    store
        .expect_partnote_all_by_part()
        .returning(|_| Ok(vec![]));

    let outcome = read(ApiRead::User, &test_session(), &mut store).await?;
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    // The full user record — the bare `User` projection.
    assert_eq!(state.user, user_row(me));
    // The `Summary` projection: the canned parts, activities, and usages.
    assert_eq!(state.summary.parts.len(), 2);
    assert_eq!(state.summary.activities.len(), 2);
    assert_eq!(state.summary.usages.len(), 2);
    assert!(state.summary.shops.is_empty());
    assert!(state.summary.users.is_empty());
    assert!(state.summary.services.is_empty());
    assert!(state.summary.plans.is_empty());
    assert!(state.summary.part_notes.is_empty());
    assert!(state.summary.attachments.is_empty());
    Ok(())
}

/// `ApiRead::User` read in a shop context (the session carries the shop)
/// filters the summary to the shop's parts — the `?shop=` context of
/// `GET /user/summary` — while the user record is unchanged.
#[tokio::test]
async fn user_read_shop_filter_limits_parts() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let shop = ShopId::from(9);
    store.expect_user_get().returning(move |_| Ok(user_row(me)));
    store.expect_get_all().returning(|_| Ok(vec![]));
    let s = shop_row(shop, me, "Workshop", true);
    let all_shops = vec![s.clone()];
    store
        .expect_shops_get_all_for_user()
        .returning(move |_| Ok(all_shops.clone()));
    // The caller owns the shop: the owner is reused (no re-read) and there
    // are no subscribers to fetch.
    store
        .expect_subscriptions_for_shop()
        .returning(|_| Ok(vec![]));
    // The shop-scoped read: the summary is filtered to the shop's parts —
    // the one registered part — via the shop's owner check and part list.
    let shop_get = s.clone();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_get.clone()));
    let registered = Part {
        shop: Some(shop),
        ..part_row(PartId::from(13), me, CHAIN)
    };
    store
        .expect_shop_get_parts()
        .returning(move |_| Ok(vec![registered.clone()]));
    store.expect_by_user().returning(|_| Ok(vec![]));
    store
        .expect_usage_get()
        .returning(|id| Ok(Some(usage_row(id))));
    store
        .expect_attachments_all_by_part()
        .returning(|_| Ok(vec![]));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_by_part().returning(|_| Ok(vec![]));
    store
        .expect_partnote_all_by_part()
        .returning(|_| Ok(vec![]));

    // The acting session carries the shop (spec #517, ticket #588): the read
    // filters the summary to the session's shop, not an in-message parameter.
    let shop_session = mock_session_with_shop(me, shop);
    let outcome = read(ApiRead::User, &shop_session, &mut store).await?;
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert_eq!(state.user.id, me);
    // The summary is filtered to the shop's parts — the one registered part,
    // with its shop set — and to the shop itself.
    assert_eq!(state.summary.parts.len(), 1);
    assert_eq!(
        state.summary.parts.get(&PartId::from(13)).unwrap().shop,
        Some(shop)
    );
    assert_eq!(state.summary.shops.len(), 1);
    assert_eq!(state.summary.shops.get(&shop).unwrap().name, "Workshop");
    Ok(())
}

/// A `User` read of a missing user is a `NotFound`.
#[tokio::test]
async fn user_read_missing_is_not_found() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_user_get()
        .returning(|_| Err(Error::NotFound("user".into())));
    let err = read(ApiRead::User, &test_session(), &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// --- Activity ---

/// `ApiRead::Activity` reads the user's activity by id, with its access check.
#[tokio::test]
async fn activity_read_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let act = activity_row(ActivityId::new(1), me, "Morning Ride");
    let act_read = act.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(act_read.clone())));

    let outcome = read(
        ApiRead::Activity(ActivityId::new(1)),
        &test_session(),
        &mut store,
    )
    .await?;
    let ReadOutcome::Activity(got) = outcome else {
        panic!("the Activity read returns the Activity");
    };
    assert_eq!(got.id, ActivityId::new(1));
    assert_eq!(got.name, "Morning Ride");
    assert_eq!(got.user_id, me);
    Ok(())
}

/// A missing activity is a `NotFound`.
#[tokio::test]
async fn activity_read_missing_is_not_found() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_activity_read_by_id().returning(|_| Ok(None));
    let err = read(
        ApiRead::Activity(ActivityId::new(999_999)),
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

/// An activity the session's user does not own is a `Forbidden` — the read
/// checks authorization, like the handler's op does.
#[tokio::test]
async fn activity_read_foreign_is_forbidden() -> TbResult<()> {
    let mut store = MockStore::new();
    // The activity belongs to user 1; user 2 may not read it.
    let act = activity_row(ActivityId::new(1), test_user(), "Morning Ride");
    let act_read = act.clone();
    store
        .expect_activity_read_by_id()
        .returning(move |_| Ok(Some(act_read.clone())));

    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::Activity(ActivityId::new(1)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

// --- Part ---

/// `ApiRead::Part` reads the user's part by id, with its access check.
#[tokio::test]
async fn part_read_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(PartId::from(1), me, BIKE)));

    let outcome = read(ApiRead::Part(PartId::from(1)), &test_session(), &mut store).await?;
    let ReadOutcome::Part(part) = outcome else {
        panic!("the Part read returns the Part");
    };
    assert_eq!(part.id, PartId::from(1));
    assert_eq!(part.owner, me);
    Ok(())
}

/// A missing part is a `NotFound`, and a part the session's user does not
/// own (with no shop session) is a `Forbidden`.
#[tokio::test]
async fn part_read_errors() -> TbResult<()> {
    let mut store = MockStore::new();
    // A missing part is NotFound; a part owned by user 1 is Forbidden to user 2.
    store.expect_partid_get_part().returning(|id| {
        if id == PartId::from(9999) {
            Err(Error::NotFound("part".into()))
        } else {
            Ok(part_row(id, test_user(), CHAIN))
        }
    });

    let err = read(
        ApiRead::Part(PartId::from(9999)),
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // Part 1 belongs to user 1; user 2 (no shop session) may not read it.
    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::Part(PartId::from(1)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::PartCategories` returns the main part types of the user's
/// parts — the op's rule: only types with `is_main`, so the bike.
#[tokio::test]
async fn part_categories_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    store.expect_part_get_all_for_userid().returning(move |_| {
        Ok(vec![
            part_row(PartId::from(1), test_user(), BIKE),
            part_row(PartId::from(4), test_user(), CHAIN),
        ])
    });

    let outcome = read(ApiRead::PartCategories, &test_session(), &mut store).await?;
    let ReadOutcome::PartCategories(categories) = outcome else {
        panic!("the PartCategories read returns the category set");
    };
    assert_eq!(categories, HashSet::from([BIKE])); // bikes — the only main type
    Ok(())
}

// --- PartNote ---

/// `ApiRead::PartNotes` lists the user's notes for a part, with the part's
/// access check.
#[tokio::test]
async fn part_notes_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let part = PartId::from(13);
    // The access check reads the part (owner = me), then the note list.
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, me, CHAIN)));
    let note = note_row(PartNoteId::from(1), part, "Check the tension");
    let note_list = note.clone();
    store
        .expect_partnote_all_by_part()
        .returning(move |_| Ok(vec![note_list.clone()]));

    let outcome = read(ApiRead::PartNotes(part), &test_session(), &mut store).await?;
    let ReadOutcome::PartNotes(notes) = outcome else {
        panic!("the PartNotes read returns the note list");
    };
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, note.id);
    assert_eq!(notes[0].name, "Check the tension");
    assert_eq!(notes[0].part, part);
    Ok(())
}

/// Notes of a part the session's user does not own are a `Forbidden`, and a
/// missing part is a `NotFound`.
#[tokio::test]
async fn part_notes_errors() -> TbResult<()> {
    let mut store = MockStore::new();
    // A missing part is NotFound; a part owned by user 1 is Forbidden to user 2.
    store.expect_partid_get_part().returning(|id| {
        if id == PartId::from(9999) {
            Err(Error::NotFound("part".into()))
        } else {
            Ok(part_row(id, test_user(), CHAIN))
        }
    });

    let err = read(
        ApiRead::PartNotes(PartId::from(9999)),
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // Part 13 belongs to user 1; user 2 may not list its notes.
    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::PartNotes(PartId::from(13)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// A note the session's user has no access to (foreign part) is a
/// `Forbidden`, and a missing note is a `NotFound`.
#[tokio::test]
async fn part_note_file_errors() -> TbResult<()> {
    let mut store = MockStore::new();
    let part = PartId::from(13);
    let note_id = PartNoteId::from(7);
    // A missing note is NotFound; a note on a part owned by user 1 is
    // Forbidden to user 2 (the access check reads the note, then its part).
    store.expect_partnote_get().returning(move |id| {
        if id == PartNoteId::from(9999) {
            Err(Error::NotFound("note".into()))
        } else {
            Ok(note_row(id, part, "Text"))
        }
    });
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user(), CHAIN)));

    let err = read(
        ApiRead::PartNoteFile(PartNoteId::from(9999)),
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // A note on user 1's part 13; user 2 has no access to it.
    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::PartNoteFile(note_id), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

// --- Shop ---

/// `ShopParts` of a shop the session's user has no access to is a
/// `Forbidden`, and a missing shop is a `NotFound`.
#[tokio::test]
async fn shop_parts_read_errors() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let shop = ShopId::from(9);
    // A missing shop is NotFound; a shop owned by user 1 is Forbidden to user 2.
    store.expect_shop_get().returning(move |id| {
        if id == ShopId::from(9999) {
            Err(Error::NotFound("shop".into()))
        } else {
            Ok(shop_row(id, me, "Workshop", true))
        }
    });
    // User 2 is neither owner nor subscriber of the shop.
    store
        .expect_subscription_find_active()
        .returning(|_, _| Ok(None));

    let err = read(
        ApiRead::ShopParts(ShopId::from(9999)),
        &test_session(),
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::ShopParts(shop), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::ShopSearch` searches shops by the `?q=` query string and
/// returns the shops plus their owners' public user records (the
/// `UserPublic` projection: the admin flag and onboarding status dropped).
#[tokio::test]
async fn shop_search_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let other = UserId::from(2);
    let workshop = shop_row(ShopId::from(1), me, "Workshop", true);
    let other_shop = shop_row(ShopId::from(2), other, "Other Shop", true);
    store.expect_shops_search().returning(move |q| {
        if q == "Workshop" {
            Ok(vec![workshop.clone()])
        } else {
            // An empty query matches everything the store's search matches.
            Ok(vec![workshop.clone(), other_shop.clone()])
        }
    });
    store.expect_user_get().returning(move |id| {
        if id == other {
            Ok(User {
                name: "Other".to_string(),
                firstname: "Cyclist".to_string(),
                ..user_row(id)
            })
        } else {
            Ok(user_row(id))
        }
    });
    store
        .expect_subscriptions_for_shop()
        .returning(|_| Ok(vec![]));

    let outcome = read(
        ApiRead::ShopSearch("Workshop".to_string()),
        &test_session(),
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSearch { shops, users } = outcome else {
        panic!("the ShopSearch read returns the shops plus their users");
    };
    assert_eq!(shops.len(), 1);
    assert_eq!(shops[0].id, ShopId::from(1));
    assert_eq!(shops[0].name, "Workshop");
    // The owner's public record, with the admin flag and onboarding status
    // dropped (the `UserPublic` projection).
    assert_eq!(
        users,
        vec![UserPublic {
            id: me,
            name: "Bike".to_string(),
            firstname: "Tenda".to_string(),
            avatar: None,
        }]
    );

    // An empty query matches everything the store's search matches (both shops).
    let outcome = read(
        ApiRead::ShopSearch(String::new()),
        &test_session(),
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSearch { shops, .. } = outcome else {
        panic!("the ShopSearch read returns the shops plus their users");
    };
    assert_eq!(shops.len(), 2);
    Ok(())
}

/// `ApiRead::ShopSubscriptions(Some(shop))` is the shop's route: the pending
/// subscriptions for that shop, owner only — the op filters to pending.
#[tokio::test]
async fn shop_subscriptions_for_shop_dispatches() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let active_shop = ShopId::from(1);
    let pending_shop = ShopId::from(2);
    store.expect_shop_get().returning(move |id| {
        if id == active_shop {
            Ok(shop_row(id, me, "Workshop", true))
        } else {
            Ok(shop_row(id, me, "Slow Shop", false))
        }
    });
    // One auto-approved (active) and one pending subscription.
    store.expect_subscriptions_for_shop().returning(move |id| {
        if id == active_shop {
            Ok(vec![sub_row(
                SubscriptionId::from(1),
                active_shop,
                UserId::from(2),
                SubscriptionStatus::Active,
            )])
        } else {
            Ok(vec![sub_row(
                SubscriptionId::from(2),
                pending_shop,
                UserId::from(2),
                SubscriptionStatus::Pending,
            )])
        }
    });

    // The op filters to pending — the auto-approved subscription is active,
    // so the owner sees none.
    let outcome = read(
        ApiRead::ShopSubscriptions(Some(active_shop)),
        &test_session(),
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    assert!(
        subs.is_empty(),
        "the auto-approved subscription is not pending"
    );

    // A pending subscription is visible to the owner.
    let outcome = read(
        ApiRead::ShopSubscriptions(Some(pending_shop)),
        &test_session(),
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].id, SubscriptionId::from(2));
    assert_eq!(subs[0].status, SubscriptionStatus::Pending);
    assert_eq!(subs[0].shop.name, "Slow Shop");
    Ok(())
}

/// `ShopSubscriptions(Some(shop))` for a shop the session's user does not
/// own is a `Forbidden`.
#[tokio::test]
async fn shop_subscriptions_for_shop_foreign_is_forbidden() -> TbResult<()> {
    let mut store = MockStore::new();
    let me = test_user();
    let shop = ShopId::from(9);
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, me, "Workshop", true)));

    let session = mock_session(UserId::from(2));
    let err = read(ApiRead::ShopSubscriptions(Some(shop)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}
