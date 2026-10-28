use super::*;
use crate::test_support::MockStore;
use crate::test_support::fixtures::sample_purchase_date;
use crate::{Part, PartId, PartTypeId, UsageId};

/// A canned `Shop` row: the minimal shape the domain logic needs (id, owner,
/// name, auto_approve).
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

/// A canned `ShopSubscription` row: the minimal shape the domain logic needs.
fn sub_row(
    id: SubscriptionId,
    shop_id: ShopId,
    user_id: UserId,
    status: SubscriptionStatus,
    message: Option<String>,
    response_message: Option<String>,
) -> ShopSubscription {
    let t = sample_purchase_date();
    ShopSubscription {
        id,
        shop_id,
        user_id,
        status,
        message,
        response_message,
        created_at: t,
        updated_at: t,
    }
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner).
fn part_row(id: PartId, owner: UserId) -> Part {
    Part {
        id,
        owner,
        what: PartTypeId::from(1),
        name: "Test Bike".to_string(),
        vendor: "Trek".to_string(),
        model: "Marlin".to_string(),
        purchase: sample_purchase_date(),
        last_used: sample_purchase_date(),
        disposed_at: None,
        usage: UsageId::new(),
        source: None,
        shop: None,
    }
}

// === Create ===

/// Auto-approve shops create the subscription, then flip it to Active with a
/// welcome message: the domain logic runs over the canned store rows.
#[tokio::test]
async fn subscription_create_auto_approve_active() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let welcome = "Welcome to Bike Barn".to_string();
    let pending = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );
    let active = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        Some(welcome.clone()),
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", true)));
    store
        .expect_subscription_find_pending()
        .returning(|_, _| Ok(None));
    store
        .expect_subscription_find_active()
        .returning(|_, _| Ok(None));
    store
        .expect_subscription_create()
        .returning(move |_, _, _| Ok(pending.clone()));
    store
        .expect_subscription_approve()
        .returning(move |_, _, _| Ok(active.clone()));

    let created = SubscriptionId::create(shop, None, subscriber, &mut store).await?;

    assert_eq!(created.status, SubscriptionStatus::Active);
    assert_eq!(created.response_message, Some(welcome));
    Ok(())
}

/// A pending subscription already on the (shop, user) pair is a conflict.
#[tokio::test]
async fn subscription_create_dup_pending_conflict() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let existing = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, UserId::from(2), "Bike Barn", false)));
    store
        .expect_subscription_find_pending()
        .returning(move |_, _| Ok(Some(existing.clone())));

    let result = SubscriptionId::create(shop, None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
    Ok(())
}

/// An active subscription already on the (shop, user) pair is a conflict.
#[tokio::test]
async fn subscription_create_dup_active_conflict() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let active = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        None,
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, UserId::from(2), "Bike Barn", false)));
    store
        .expect_subscription_find_pending()
        .returning(|_, _| Ok(None));
    store
        .expect_subscription_find_active()
        .returning(move |_, _| Ok(Some(active.clone())));

    let result = SubscriptionId::create(shop, None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
    Ok(())
}

// === checkuser ===

/// The subscriber can access their own subscription.
#[tokio::test]
async fn subscription_checkuser_subscriber_ok() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));

    let result = sub.id.checkuser(subscriber, &mut store).await;
    assert!(result.is_ok());
    Ok(())
}

/// The shop owner can access a subscription for their shop.
#[tokio::test]
async fn subscription_checkuser_shop_owner_ok() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));

    let result = sub.id.checkuser(owner, &mut store).await;
    assert!(result.is_ok());
    Ok(())
}

/// A stranger (neither subscriber nor shop owner) is forbidden.
#[tokio::test]
async fn subscription_checkuser_stranger_forbidden() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let stranger = UserId::from(3);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));

    let result = sub.id.checkuser(stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === approve / reject ===

/// Only the shop owner can approve: a non-owner is forbidden, the owner
/// approves and the domain logic flips the status to Active.
#[tokio::test]
async fn subscription_approve_owner_only() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));
    store
        .expect_subscription_approve()
        .returning(move |_id, _status, msg| {
            Ok(sub_row(
                SubscriptionId::from(1),
                shop,
                subscriber,
                SubscriptionStatus::Active,
                None,
                msg,
            ))
        });

    // Non-owner cannot approve
    let denied = sub.id.approve(None, subscriber, &mut store).await;
    assert!(matches!(denied, Err(Error::Forbidden(_))));

    // Owner can approve
    let approved = sub
        .id
        .approve(Some("Welcome!".to_string()), owner, &mut store)
        .await?;
    assert_eq!(approved.status, SubscriptionStatus::Active);
    assert_eq!(approved.response_message, Some("Welcome!".into()));
    Ok(())
}

/// Approve is a no-op on an already-active (auto-approved) subscription: a
/// conflict, not a re-approval.
#[tokio::test]
async fn subscription_approve_pending_only() -> TbResult<()> {
    let shop = ShopId::from(1);
    let owner = UserId::from(2);
    // Already active (auto-approved)
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        owner,
        SubscriptionStatus::Active,
        None,
        Some("Welcome to Bike Barn".into()),
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", true)));

    let result = sub.id.approve(None, owner, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
    Ok(())
}

/// Only the shop owner can reject; the domain logic flips the status to
/// Rejected with the response message.
#[tokio::test]
async fn subscription_reject_owner_only() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));
    store
        .expect_subscription_approve()
        .returning(move |_id, _status, msg| {
            Ok(sub_row(
                SubscriptionId::from(1),
                shop,
                subscriber,
                SubscriptionStatus::Rejected,
                None,
                msg,
            ))
        });

    let rejected = sub
        .id
        .reject(Some("Nope".to_string()), owner, &mut store)
        .await?;
    assert_eq!(rejected.status, SubscriptionStatus::Rejected);
    assert_eq!(rejected.response_message, Some("Nope".into()));
    Ok(())
}

/// A non-owner cannot reject a subscription.
#[tokio::test]
async fn subscription_reject_non_owner_forbidden() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));

    let result = sub.id.reject(None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === cancel ===

/// Only the subscriber can cancel: the shop owner is forbidden, the
/// subscriber cancels, and the (deleted) subscription is no longer readable.
#[tokio::test]
async fn subscription_cancel_subscriber_only() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    // cancel(owner) reads the sub then denies; cancel(subscriber) reads it
    // again then deletes it; read(subscriber) then hits the deleted row.
    // The same `subscription_get` is called three times with three outcomes,
    // so a stateful counter drives the canned responses in call order.
    let sub_owned = sub.clone();
    let counter = std::sync::Arc::new(std::sync::Mutex::new(0));
    let counter_calls = counter.clone();
    store.expect_subscription_get().returning(move |_| {
        let n = *counter_calls.lock().unwrap();
        *counter_calls.lock().unwrap() += 1;
        match n {
            0 | 1 => Ok(sub_owned.clone()),
            _ => Err(Error::NotFound("subscription".into())),
        }
    });
    store.expect_shop_get_parts().returning(|_| Ok(vec![]));
    store.expect_subscription_delete().returning(|_| Ok(()));

    // Shop owner cannot cancel
    let denied = sub.id.cancel(owner, &mut store).await;
    assert!(matches!(denied, Err(Error::Forbidden(_))));

    // Subscriber can cancel
    sub.id.cancel(subscriber, &mut store).await?;
    assert!(sub.id.read(subscriber, &mut store).await.is_err());
    Ok(())
}

/// A subscriber can cancel an active subscription when they have no parts in
/// the shop.
#[tokio::test]
async fn subscription_cancel_active_ok() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        Some("Welcome to Bike Barn".into()),
    );

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store.expect_shop_get_parts().returning(|_| Ok(vec![]));
    store.expect_subscription_delete().returning(|_| Ok(()));

    sub.id.cancel(subscriber, &mut store).await?;
    Ok(())
}

/// A subscriber with parts still registered in the shop cannot cancel: a
/// conflict.
#[tokio::test]
async fn subscription_cancel_conflict_parts_in_shop() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        Some("Welcome to Bike Barn".into()),
    );
    let bike = part_row(PartId::from(1), subscriber);

    let mut store = MockStore::new();
    let sub_owned = sub.clone();
    store
        .expect_subscription_get()
        .returning(move |_| Ok(sub_owned.clone()));
    store
        .expect_shop_get_parts()
        .returning(move |_| Ok(vec![bike.clone()]));

    let result = sub.id.cancel(subscriber, &mut store).await;
    assert!(matches!(&result, Err(Error::Conflict(msg)) if msg.contains("parts in the shop")));
    Ok(())
}

// === queries ===

/// The shop owner sees only the pending subscriptions for their shop, each
/// with its shop details.
#[tokio::test]
async fn subscription_get_pending_for_shop() -> TbResult<()> {
    let shop = ShopId::from(1);
    let owner = UserId::from(2);
    let subscriber = UserId::from(1);
    let s2_user = UserId::from(4);
    // one active (filtered out), one pending (kept)
    let active = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        None,
    );
    let pending = sub_row(
        SubscriptionId::from(2),
        shop,
        s2_user,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));
    store
        .expect_subscriptions_for_shop()
        .returning(move |_| Ok(vec![active.clone(), pending.clone()]));

    let pending_list = ShopSubscription::get_pending_for_shop(shop, owner, &mut store).await?;
    assert_eq!(pending_list.len(), 1);
    assert_eq!(pending_list[0].user_id, s2_user);
    assert_eq!(pending_list[0].shop.name, "Bike Barn");
    Ok(())
}

/// `with_shop_details` maps each subscription to its shop details.
#[tokio::test]
async fn subscription_with_shop_details() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let sub = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Pending,
        None,
        None,
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, UserId::from(2), "Bike Barn", false)));

    let details = ShopSubscription::with_shop_details(vec![sub], &mut store).await?;
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].shop.id, shop);
    assert_eq!(details[0].shop.name, "Bike Barn");
    Ok(())
}

/// Read access to a shop passes for an active subscriber (not the owner).
#[tokio::test]
async fn subscription_check_active_ok() -> TbResult<()> {
    let shop = ShopId::from(1);
    let subscriber = UserId::from(1);
    let owner = UserId::from(2);
    let active = sub_row(
        SubscriptionId::from(1),
        shop,
        subscriber,
        SubscriptionStatus::Active,
        None,
        None,
    );

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, owner, "Bike Barn", false)));
    store
        .expect_subscription_find_active()
        .returning(move |_, _| Ok(Some(active.clone())));

    // `check` is pub(super), exercised here via get_for_read, which calls it.
    ShopId::get_for_read(shop.into(), subscriber, &mut store).await?;
    Ok(())
}
