use tb_domain::*;
use tb_domain::{PartStore, PartTypeId, ShopStore, UsageId};
use tb_teststore::TestStore;
use time::OffsetDateTime;

async fn setup() -> (TestStore, UserId, UserId, Shop) {
    let mut store = TestStore::new();
    let owner = UserId::create("Shop", "Owner", &None, &mut store)
        .await
        .unwrap();
    let subscriber = UserId::create("Ride", "Fan", &None, &mut store)
        .await
        .unwrap();
    let shop = ShopId::create("Bike Barn".into(), None, false, owner, &mut store)
        .await
        .unwrap();
    (store, owner, subscriber, shop)
}

async fn setup_auto_approve() -> (TestStore, UserId, UserId, Shop) {
    let mut store = TestStore::new();
    let owner = UserId::create("Shop", "Owner", &None, &mut store)
        .await
        .unwrap();
    let subscriber = UserId::create("Ride", "Fan", &None, &mut store)
        .await
        .unwrap();
    let shop = ShopId::create("Auto Shop".into(), None, true, owner, &mut store)
        .await
        .unwrap();
    (store, owner, subscriber, shop)
}

// === Create ===

#[tokio::test]
async fn subscription_create_fresh_pending() {
    let (mut store, _, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, Some("Hi!".into()), subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Pending);
    assert_eq!(sub.shop_id, shop.id);
    assert_eq!(sub.user_id, subscriber);
    assert_eq!(sub.message, Some("Hi!".into()));
}

#[tokio::test]
async fn subscription_create_auto_approve_active() {
    let (mut store, _, subscriber, shop) = setup_auto_approve().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);
    assert!(sub.response_message.is_some());
}

#[tokio::test]
async fn subscription_create_dup_pending_conflict() {
    let (mut store, _, subscriber, shop) = setup().await;
    SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let result = SubscriptionId::create(shop.id, None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn subscription_create_dup_active_conflict() {
    let (mut store, _, subscriber, shop) = setup_auto_approve().await;
    SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let result = SubscriptionId::create(shop.id, None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
}

// === checkuser ===

#[tokio::test]
async fn subscription_checkuser_subscriber_ok() {
    let (mut store, _, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let result = sub.id.checkuser(subscriber, &mut store).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn subscription_checkuser_shop_owner_ok() {
    let (mut store, owner, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let result = sub.id.checkuser(owner, &mut store).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn subscription_checkuser_stranger_forbidden() {
    let (mut store, owner, _, shop) = setup().await;
    let stranger = UserId::create("Eve", "X", &None, &mut store).await.unwrap();
    let sub = SubscriptionId::create(shop.id, None, owner, &mut store)
        .await
        .unwrap();
    let result = sub.id.checkuser(stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

// === approve / reject ===

#[tokio::test]
async fn subscription_approve_owner_only() {
    let (mut store, owner, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();

    // Non-owner cannot approve
    let result = sub.id.approve(None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));

    // Owner can approve
    let approved = sub
        .id
        .approve(Some("Welcome!".to_string()), owner, &mut store)
        .await
        .unwrap();
    assert_eq!(approved.status, SubscriptionStatus::Active);
    assert_eq!(approved.response_message, Some("Welcome!".into()));
}

#[tokio::test]
async fn subscription_approve_pending_only() {
    let (mut store, owner, _, shop) = setup_auto_approve().await;
    let sub = SubscriptionId::create(shop.id, None, owner, &mut store)
        .await
        .unwrap();
    // Already active (auto-approved)
    let result = sub.id.approve(None, owner, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn subscription_reject_owner_only() {
    let (mut store, owner, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();

    let rejected = sub
        .id
        .reject(Some("Nope".to_string()), owner, &mut store)
        .await
        .unwrap();
    assert_eq!(rejected.status, SubscriptionStatus::Rejected);
    assert_eq!(rejected.response_message, Some("Nope".into()));
}

#[tokio::test]
async fn subscription_reject_non_owner_forbidden() {
    let (mut store, _, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let result = sub.id.reject(None, subscriber, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

// === cancel ===

#[tokio::test]
async fn subscription_cancel_subscriber_only() {
    let (mut store, owner, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();

    // Shop owner cannot cancel
    let result = sub.id.cancel(owner, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));

    // Subscriber can cancel
    sub.id.cancel(subscriber, &mut store).await.unwrap();
    assert!(sub.id.read(subscriber, &mut store).await.is_err());
}

#[tokio::test]
async fn subscription_cancel_active_ok() {
    let (mut store, _, subscriber, shop) = setup_auto_approve().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);
    sub.id.cancel(subscriber, &mut store).await.unwrap();
}

#[tokio::test]
async fn subscription_cancel_conflict_parts_in_shop() {
    let (mut store, _, subscriber, shop) = setup_auto_approve().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);

    // Register a part owned by subscriber in the shop
    let now = OffsetDateTime::now_utc();
    store
        .part_create(
            PartTypeId::from(1),
            "Test Bike".into(),
            "Trek".into(),
            "Marlin".into(),
            now,
            None,
            UsageId::new(),
            subscriber,
            Some(shop.id),
        )
        .await
        .unwrap();

    let result = sub.id.cancel(subscriber, &mut store).await;
    assert!(matches!(&result, Err(Error::Conflict(msg)) if msg.contains("parts in the shop")));
}

// === queries ===

#[tokio::test]
async fn subscription_get_pending_for_shop() {
    let (mut store, owner, subscriber, shop) = setup().await;
    let s1 = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    // Make it active so it doesn't show in pending
    store
        .subscription_approve(s1.id, SubscriptionStatus::Active, None)
        .await
        .unwrap();

    let s2_user = UserId::create("U2", "L2", &None, &mut store).await.unwrap();
    SubscriptionId::create(shop.id, None, s2_user, &mut store)
        .await
        .unwrap();

    let pending = ShopSubscription::get_pending_for_shop(shop.id, owner, &mut store)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].user_id, s2_user);
    assert_eq!(pending[0].shop.name, "Bike Barn");
}

#[tokio::test]
async fn subscription_get_for_user() {
    let (mut store, _, subscriber, shop) = setup().await;
    SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();

    let subs = ShopSubscription::get_for_user(subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].shop_id, shop.id);
}

#[tokio::test]
async fn subscription_with_shop_details() {
    let (mut store, _, subscriber, shop) = setup().await;
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();

    let details = ShopSubscription::with_shop_details(vec![sub], &mut store)
        .await
        .unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].shop.id, shop.id);
    assert_eq!(details[0].shop.name, "Bike Barn");
}

#[tokio::test]
async fn subscription_check_active_ok() {
    let (mut store, _, subscriber, shop) = setup_auto_approve().await;
    SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    // check is pub(super), call via get_for_read which uses it internally
    ShopId::get_for_read(shop.id.into(), subscriber, &mut store)
        .await
        .unwrap();
}

#[tokio::test]
async fn subscription_get_for_shop() {
    let (mut store, _, subscriber, shop) = setup().await;
    let _s1 = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    let s2_user = UserId::create("U2", "L2", &None, &mut store).await.unwrap();
    SubscriptionId::create(shop.id, None, s2_user, &mut store)
        .await
        .unwrap();

    let subs = ShopSubscription::get_for_shop(shop.id, &mut store)
        .await
        .unwrap();
    assert_eq!(subs.len(), 2);
}
