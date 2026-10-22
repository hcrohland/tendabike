use super::*;
use crate::test_support::{
    MemStore, TestSession,
    fixtures::{test_session, test_user},
};
use crate::{
    ActivityId, Error, OnboardingStatus, PartId, PartNoteId, PartTypeId, ShopId, SubscriptionId,
    SubscriptionStatus, User, UserId, UserPublic,
};
use std::collections::HashSet;
use time::OffsetDateTime;

// --- User (the unified read: UserState = User + Summary) ---

/// `ApiRead::User` is the unified user read (spec #558 §3.5): the
/// `UserState` composite — the full `User` plus the `Summary` of everything
/// the user owns (the session here carries no shop). The fixture content is
/// the same snapshot the in-memory `user_summary_read` asserts.
#[tokio::test]
async fn user_read_returns_user_state() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let outcome = read(ApiRead::User, &session, &mut store).await?;
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    // The full user record — the bare `User` projection.
    assert_eq!(
        state.user,
        User {
            id: test_user(),
            name: "Bike".to_string(),
            firstname: "Tenda".to_string(),
            avatar: None,
            is_admin: false,
            onboarding_status: OnboardingStatus::Pending,
        }
    );
    // The `Summary` projection: the full fixture content.
    assert_eq!(state.summary.parts.len(), 17);
    assert_eq!(state.summary.activities.len(), 3);
    assert_eq!(state.summary.attachments.len(), 11);
    assert_eq!(state.summary.usages.len(), 28);
    assert!(state.summary.shops.is_empty());
    assert!(state.summary.users.is_empty());
    assert!(state.summary.services.is_empty());
    assert!(state.summary.plans.is_empty());
    assert!(state.summary.part_notes.is_empty());
    Ok(())
}

/// `ApiRead::User` read in a shop context (the session carries the shop)
/// filters the summary to the shop's parts — the `?shop=` context of
/// `GET /user/summary` — while the user record is unchanged.
#[tokio::test]
async fn user_read_shop_filter_limits_parts() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    // A shop the user owns, with one of the user's parts registered.
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    // The route's checkuser requires an (owner) subscription, so create one
    // first — auto-approve activates it.
    SubscriptionId::create(shop.id, None, session.user_id(), &mut store).await?;
    let part = PartId::from(13); // "Spare Chain 1" — loose, owned by user 1
    shop.id.register_part(part, &session, &mut store).await?;

    // The acting session carries the shop (spec #517, ticket #588): the read
    // filters the summary to the session's shop, not an in-message parameter.
    let shop_session = TestSession::with_shop(session.user_id(), shop.id);
    let outcome = read(ApiRead::User, &shop_session, &mut store).await?;
    let ReadOutcome::User(state) = outcome else {
        panic!("the User read returns the UserState composite");
    };
    assert_eq!(state.user.id, test_user());
    // The summary is filtered to the shop's parts — the one registered
    // part, with its usages/attachments — and to the shop itself.
    assert_eq!(state.summary.parts.len(), 1);
    assert_eq!(state.summary.parts.get(&part).unwrap().shop, Some(shop.id));
    assert_eq!(state.summary.shops.len(), 1);
    assert_eq!(state.summary.shops.get(&shop.id).unwrap().name, "Workshop");
    Ok(())
}

/// A `User` read of a missing user is a `NotFound` on the in-memory store.
#[tokio::test]
async fn user_read_missing_is_not_found() -> TbResult<()> {
    let mut store = MemStore::new();
    let session = test_session();
    let err = read(ApiRead::User, &session, &mut store).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

// --- Activity ---

/// `ApiRead::Activity` reads the user's activity by id — the fixture's
/// first ride, with its access check.
#[tokio::test]
async fn activity_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let outcome = read(ApiRead::Activity(ActivityId::new(1)), &session, &mut store).await?;
    let ReadOutcome::Activity(act) = outcome else {
        panic!("the Activity read returns the Activity");
    };
    assert_eq!(act.id, ActivityId::new(1));
    assert_eq!(act.name, "Morning Ride");
    assert_eq!(act.user_id, test_user());
    Ok(())
}

/// A missing activity is a `NotFound`.
#[tokio::test]
async fn activity_read_missing_is_not_found() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let err = read(
        ApiRead::Activity(ActivityId::new(999_999)),
        &session,
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
    let mut store = MemStore::prepopulated();
    // Part 1 (and the fixture activities) belong to user 1; user 2 may not
    // read them.
    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::Activity(ActivityId::new(1)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

// --- Part ---

/// `ApiRead::Part` reads the user's part by id — the fixture's "Main Bike",
/// with its access check.
#[tokio::test]
async fn part_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let outcome = read(ApiRead::Part(PartId::from(1)), &session, &mut store).await?;
    let ReadOutcome::Part(part) = outcome else {
        panic!("the Part read returns the Part");
    };
    assert_eq!(part.id, PartId::from(1));
    assert_eq!(part.name, "Main Bike");
    assert_eq!(part.owner, test_user());
    Ok(())
}

/// A missing part is a `NotFound`, and a part the session's user does not
/// own (with no shop session) is a `Forbidden`.
#[tokio::test]
async fn part_read_errors() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let err = read(ApiRead::Part(PartId::from(9999)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // Part 1 belongs to user 1; user 2 (no shop session) may not read it.
    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::Part(PartId::from(1)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::PartCategories` returns the main part types of the user's
/// parts — the op's rule: only types with `is_main`, so the fixture's
/// single bike type.
#[tokio::test]
async fn part_categories_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let outcome = read(ApiRead::PartCategories, &session, &mut store).await?;
    let ReadOutcome::PartCategories(categories) = outcome else {
        panic!("the PartCategories read returns the category set");
    };
    assert_eq!(
        categories,
        HashSet::from([PartTypeId::from_id(1)]), // bikes — the only main type in the fixture
    );
    Ok(())
}

// --- PartNote ---

/// `ApiRead::PartNotes` lists the user's notes for a part — created through
/// the same domain op the create route uses, with the part's access check.
#[tokio::test]
async fn part_notes_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let part = PartId::from(13);
    let note = part
        .note_create_text(
            &session,
            "Check the tension".to_string(),
            OffsetDateTime::now_utc(),
            &mut store,
        )
        .await?;

    let outcome = read(ApiRead::PartNotes(part), &session, &mut store).await?;
    let ReadOutcome::PartNotes(notes) = outcome else {
        panic!("the PartNotes read returns the note list");
    };
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].id, note.id);
    assert_eq!(notes[0].name, "Check the tension");
    assert_eq!(notes[0].part, part);
    Ok(())
}

/// Notes of a part the session's user does not own are a `Forbidden`, and
/// a missing part is a `NotFound`.
#[tokio::test]
async fn part_notes_errors() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let part = PartId::from(13);
    let session = test_session();
    let err = read(ApiRead::PartNotes(PartId::from(9999)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // Part 13 belongs to user 1; user 2 may not list its notes.
    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::PartNotes(part), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::PartNoteFile` returns the note plus its file bytes — the
/// download route's data, headers and encoding staying in axum. A text note
/// is a `NotFound` (its op's rule).
#[tokio::test]
async fn part_note_file_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let part = PartId::from(13);
    let data = b"file content".to_vec();
    let note = part
        .note_create_file(
            &session,
            "Manual".to_string(),
            "application/pdf".to_string(),
            Some("manual.pdf".to_string()),
            data.len() as i64,
            data.clone(),
            OffsetDateTime::now_utc(),
            &mut store,
        )
        .await?;

    let outcome = read(ApiRead::PartNoteFile(note.id), &session, &mut store).await?;
    let ReadOutcome::PartNoteFile(got_note, file) = outcome else {
        panic!("the PartNoteFile read returns the note plus its bytes");
    };
    assert_eq!(got_note.id, note.id);
    assert_eq!(got_note.name, "Manual");
    assert_eq!(file, data);

    // A text note has no file: the op's NotFound rule.
    let text = part
        .note_create_text(
            &session,
            "Text".to_string(),
            OffsetDateTime::now_utc(),
            &mut store,
        )
        .await?;
    let err = read(ApiRead::PartNoteFile(text.id), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

/// A note the session's user has no access to (foreign part) is a
/// `Forbidden`, and a missing note is a `NotFound`.
#[tokio::test]
async fn part_note_file_errors() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let err = read(
        ApiRead::PartNoteFile(PartNoteId::from(9999)),
        &session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // A note on user 1's part 13; user 2 has no access to it.
    let part = PartId::from(13);
    let note = part
        .note_create_text(
            &session,
            "Text".to_string(),
            OffsetDateTime::now_utc(),
            &mut store,
        )
        .await?;
    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::PartNoteFile(note.id), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

// --- Shop ---

/// `ApiRead::Shops` lists the user's shops — empty in the fixture, the
/// created shop once it exists.
#[tokio::test]
async fn shops_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();

    let outcome = read(ApiRead::Shops, &session, &mut store).await?;
    let ReadOutcome::Shops(shops) = outcome else {
        panic!("the Shops read returns the shop list");
    };
    assert!(shops.is_empty(), "the fixture has no shops");

    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    let outcome = read(ApiRead::Shops, &session, &mut store).await?;
    let ReadOutcome::Shops(shops) = outcome else {
        panic!("the Shops read returns the shop list");
    };
    assert_eq!(shops.len(), 1);
    assert_eq!(shops[0].id, shop.id);
    assert_eq!(shops[0].name, "Workshop");
    Ok(())
}

/// `ApiRead::Shop` reads a shop by id — every user may read a shop (the op
/// has no access check); a missing shop is a `NotFound`.
#[tokio::test]
async fn shop_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;

    let outcome = read(ApiRead::Shop(shop.id), &session, &mut store).await?;
    let ReadOutcome::Shop(got) = outcome else {
        panic!("the Shop read returns the Shop");
    };
    assert_eq!(got.id, shop.id);
    assert_eq!(got.name, "Workshop");
    assert_eq!(got.owner, test_user());

    let err = read(ApiRead::Shop(ShopId::from(9999)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    Ok(())
}

/// `ApiRead::ShopParts` lists the parts registered to a shop — the route's
/// read-access check (owner or active subscriber) plus the op's owner check.
#[tokio::test]
async fn shop_parts_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    // The register op requires an (owner) subscription; auto-approve
    // activates it.
    SubscriptionId::create(shop.id, None, session.user_id(), &mut store).await?;
    let part = PartId::from(13); // "Spare Chain 1" — loose, owned by user 1
    shop.id.register_part(part, &session, &mut store).await?;

    let outcome = read(ApiRead::ShopParts(shop.id), &session, &mut store).await?;
    let ReadOutcome::ShopParts(parts) = outcome else {
        panic!("the ShopParts read returns the part list");
    };
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].id, part);
    assert_eq!(parts[0].shop, Some(shop.id));
    Ok(())
}

/// `ShopParts` of a shop the session's user has no access to is a
/// `Forbidden`, and a missing shop is a `NotFound`.
#[tokio::test]
async fn shop_parts_read_errors() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;

    let err = read(ApiRead::ShopParts(ShopId::from(9999)), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    // User 2 is neither owner nor subscriber of the shop.
    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::ShopParts(shop.id), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::ShopSearch` searches shops by the `?q=` query string and
/// returns the shops plus their owners' public user records (and, for the
/// caller's own shops, the subscribers' records too).
#[tokio::test]
async fn shop_search_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let _ = ShopId::create(
        "Other Shop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;

    let outcome = read(
        ApiRead::ShopSearch("Workshop".to_string()),
        &session,
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSearch { shops, users } = outcome else {
        panic!("the ShopSearch read returns the shops plus their users");
    };
    assert_eq!(shops.len(), 1);
    assert_eq!(shops[0].id, shop.id);
    assert_eq!(shops[0].name, "Workshop");
    // The owner's public record, with the admin flag and onboarding status
    // dropped (the `UserPublic` projection).
    assert_eq!(users.len(), 1);
    assert_eq!(
        users[0],
        UserPublic {
            id: test_user(),
            name: "Bike".to_string(),
            firstname: "Tenda".to_string(),
            avatar: None,
        }
    );

    // An empty query matches everything the store's search matches (both
    // shops on the in-memory store).
    let outcome = read(ApiRead::ShopSearch(String::new()), &session, &mut store).await?;
    let ReadOutcome::ShopSearch { shops, .. } = outcome else {
        panic!("the ShopSearch read returns the shops plus their users");
    };
    assert_eq!(shops.len(), 2);
    Ok(())
}

/// `ApiRead::ShopSubscriptions(None)` is the mine route: the session user's
/// subscriptions with shop details.
#[tokio::test]
async fn shop_subscriptions_mine_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();

    let outcome = read(ApiRead::ShopSubscriptions(None), &session, &mut store).await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    assert!(subs.is_empty(), "the fixture has no subscriptions");

    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    let sub = SubscriptionId::create(
        shop.id,
        Some("Please approve".to_string()),
        session.user_id(),
        &mut store,
    )
    .await?;

    let outcome = read(ApiRead::ShopSubscriptions(None), &session, &mut store).await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].id, sub.id);
    assert_eq!(subs[0].shop_id, shop.id);
    assert_eq!(subs[0].shop.name, "Workshop");
    Ok(())
}

/// `ApiRead::ShopSubscriptions(Some(shop))` is the shop's route: the
/// pending subscriptions for that shop, owner only.
#[tokio::test]
async fn shop_subscriptions_for_shop_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    // User 2 (absent from the fixture's users; the op only needs the id)
    // subscribes to the shop — auto-approve activates it.
    let other = UserId::from(2);
    let sub =
        SubscriptionId::create(shop.id, Some("Let me in".to_string()), other, &mut store).await?;

    let outcome = read(
        ApiRead::ShopSubscriptions(Some(shop.id)),
        &session,
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    // The op filters to pending — the auto-approved subscription is active,
    // so the owner sees none.
    assert!(
        subs.is_empty(),
        "the auto-approved subscription is not pending"
    );
    assert_eq!(sub.status, SubscriptionStatus::Active);

    // A pending subscription (auto-approve off) is visible to the owner.
    let pending_shop = ShopId::create(
        "Slow Shop".to_string(),
        None,
        false,
        session.user_id(),
        &mut store,
    )
    .await?;
    let pending = SubscriptionId::create(pending_shop.id, None, other, &mut store).await?;
    let outcome = read(
        ApiRead::ShopSubscriptions(Some(pending_shop.id)),
        &session,
        &mut store,
    )
    .await?;
    let ReadOutcome::ShopSubscriptions(subs) = outcome else {
        panic!("the ShopSubscriptions read returns the subscriptions with details");
    };
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].id, pending.id);
    assert_eq!(subs[0].status, SubscriptionStatus::Pending);
    Ok(())
}

/// `ShopSubscriptions(Some(shop))` for a shop the session's user does not
/// own is a `Forbidden`.
#[tokio::test]
async fn shop_subscriptions_for_shop_foreign_is_forbidden() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;

    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(
        ApiRead::ShopSubscriptions(Some(shop.id)),
        &session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}

/// `ApiRead::ShopSubscription` reads one subscription by id — accessible to
/// the subscriber and to the shop's owner.
#[tokio::test]
async fn shop_subscription_read_dispatches() -> TbResult<()> {
    let mut store = MemStore::prepopulated();
    let session = test_session();
    let shop = ShopId::create(
        "Workshop".to_string(),
        None,
        true,
        session.user_id(),
        &mut store,
    )
    .await?;
    let sub = SubscriptionId::create(shop.id, None, session.user_id(), &mut store).await?;

    let outcome = read(ApiRead::ShopSubscription(sub.id), &session, &mut store).await?;
    let ReadOutcome::ShopSubscription(got) = outcome else {
        panic!("the ShopSubscription read returns the ShopSubscription");
    };
    assert_eq!(got.id, sub.id);
    assert_eq!(got.shop_id, shop.id);
    assert_eq!(got.status, SubscriptionStatus::Active);

    // A missing subscription is a NotFound, and a subscription the session's
    // user is neither subscriber of nor shop owner of is a Forbidden.
    let err = read(
        ApiRead::ShopSubscription(SubscriptionId::from(9999)),
        &session,
        &mut store,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));

    let session = crate::test_support::TestSession::new(UserId::from(2));
    let err = read(ApiRead::ShopSubscription(sub.id), &session, &mut store)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
    Ok(())
}
