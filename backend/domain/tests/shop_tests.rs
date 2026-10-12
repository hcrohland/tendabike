use tb_domain::test_support::TestSession;
use tb_domain::*;
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
    let shop = ShopId::create(
        "Bike Barn".into(),
        Some("Best shop".into()),
        false,
        owner,
        &mut store,
    )
    .await
    .unwrap();
    (store, owner, subscriber, shop)
}

// === Shop CRUD ===

#[tokio::test]
async fn shop_create_and_read() {
    let (mut store, owner, _, shop) = setup().await;
    assert_eq!(shop.name, "Bike Barn");
    assert_eq!(shop.owner, owner);
    assert!(!shop.auto_approve);

    let read = shop.id.read(&mut store).await.unwrap();
    assert_eq!(read, shop);
}

#[tokio::test]
async fn shop_get_owner_ok() {
    let (mut store, owner, _, shop) = setup().await;
    let id = ShopId::get(shop.id.into(), owner, &mut store)
        .await
        .unwrap();
    assert_eq!(id, shop.id);
}

#[tokio::test]
async fn shop_get_non_owner_forbidden() {
    let (mut store, _, stranger, shop) = setup().await;
    let result = ShopId::get(shop.id.into(), stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn shop_get_for_read_owner_ok() {
    let (mut store, owner, _, shop) = setup().await;
    let id = ShopId::get_for_read(shop.id.into(), owner, &mut store)
        .await
        .unwrap();
    assert_eq!(id, shop.id);
}

#[tokio::test]
async fn shop_get_for_read_active_subscriber_ok() {
    let (mut store, owner, subscriber, shop) = setup().await;
    // Create and manually approve a subscription
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    sub.id.approve(None, owner, &mut store).await.unwrap();

    let id = ShopId::get_for_read(shop.id.into(), subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(id, shop.id);
}

#[tokio::test]
async fn shop_get_for_read_stranger_forbidden() {
    let (mut store, _, stranger, shop) = setup().await;
    let result = ShopId::get_for_read(shop.id.into(), stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn shop_update_owner_only() {
    let (mut store, owner, _, shop) = setup().await;
    let updated = shop
        .id
        .update("New Name".into(), None, true, owner, &mut store)
        .await
        .unwrap();
    assert_eq!(updated.name, "New Name");
    assert!(updated.auto_approve);
}

#[tokio::test]
async fn shop_update_non_owner_forbidden() {
    let (mut store, _, stranger, shop) = setup().await;
    let result = shop
        .id
        .update("Hax".into(), None, false, stranger, &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn shop_delete_empty_ok() {
    let (mut store, owner, _, shop) = setup().await;
    shop.id.delete(owner, &mut store).await.unwrap();
    assert!(shop.id.read(&mut store).await.is_err());
}

#[tokio::test]
async fn shop_delete_with_bikes_conflict() {
    let (mut store, owner, _, shop) = setup().await;
    // Register a part to the shop
    let part = Part::create(
        "My Bike".into(),
        "TendaBike".into(),
        "Standard".into(),
        PartTypeId::from_id(1),
        None,
        OffsetDateTime::now_utc(),
        &TestSession::new(owner),
        &mut store,
    )
    .await
    .unwrap();
    // Use the low-level store method to set shop_id
    let parts = vec![part.id];
    store.parts_register_shop(shop.id, parts).await.unwrap();

    let result = shop.id.delete(owner, &mut store).await;
    assert!(matches!(result, Err(Error::Conflict(_))));
}

#[tokio::test]
async fn shop_search() {
    let (mut store, _, _, shop) = setup().await;
    let results = Shop::search("Bike Barn", &mut store).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, shop.id);

    let none = Shop::search("nonexistent", &mut store).await.unwrap();
    assert!(none.is_empty());
}

#[tokio::test]
async fn shop_get_all_for_user_owner_and_subscriber() {
    let (mut store, owner, subscriber, shop) = setup().await;
    // Owner sees their shop
    let owner_shops = Shop::get_all_for_user(&owner, &mut store).await.unwrap();
    assert_eq!(owner_shops.len(), 1);
    assert_eq!(owner_shops[0].id, shop.id);

    // Subscriber without active sub sees nothing
    let sub_shops = Shop::get_all_for_user(&subscriber, &mut store)
        .await
        .unwrap();
    assert!(sub_shops.is_empty());

    // After active subscription, subscriber sees the shop
    let sub = SubscriptionId::create(shop.id, None, subscriber, &mut store)
        .await
        .unwrap();
    sub.id.approve(None, owner, &mut store).await.unwrap();
    let sub_shops = Shop::get_all_for_user(&subscriber, &mut store)
        .await
        .unwrap();
    assert_eq!(sub_shops.len(), 1);
    assert_eq!(sub_shops[0].id, shop.id);
}

// === Part Registration / Unregistration (Tier C) ===

use part_type_ids::*;
use tb_domain::is_attached;
use tb_domain::test_support::part_type_ids;

/// Creates a shop with auto_approve, a shop owner, and an active subscription
/// for the customer (user 1, prepopulated). Shop owner gets the next available ID.
async fn setup_shop_and_sub(mut store: TestStore) -> (TestStore, UserId, UserId, Shop) {
    let customer = UserId::from(1);
    let shop_owner = UserId::create("Shop", "Owner", &None, &mut store)
        .await
        .unwrap();
    let shop = ShopId::create(
        "Bike Barn".into(),
        Some("Best shop".into()),
        true,
        shop_owner,
        &mut store,
    )
    .await
    .unwrap();
    let sub = SubscriptionId::create(shop.id, None, customer, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);
    (store, shop_owner, customer, shop)
}

#[tokio::test]
async fn register_part_ok() {
    let (mut store, _, customer, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::new(customer);
    let summary = shop
        .id
        .register_part(PartId::from(1), &session, &mut store)
        .await
        .unwrap();
    // Bike A + Front Wheel A + Rear Wheel A + Chain A + both tires = 6
    assert_eq!(summary.parts.len(), 6);
}

#[tokio::test]
async fn register_part_cascades_subparts() {
    let (mut store, _, customer, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::new(customer);
    let summary = shop
        .id
        .register_part(PartId::from(1), &session, &mut store)
        .await
        .unwrap();
    let shop_id = shop.id;
    for part in summary.parts.live() {
        assert_eq!(
            part.shop,
            Some(shop_id),
            "part {} should have shop",
            part.id
        );
    }
}

#[tokio::test]
async fn register_part_requires_subscription() {
    let mut store = TestStore::prepopulated();
    let customer = UserId::from(1);
    let shop_owner = UserId::create("Shop", "Owner", &None, &mut store)
        .await
        .unwrap();
    let shop = ShopId::create(
        "Bike Barn".into(),
        Some("Best shop".into()),
        false,
        shop_owner,
        &mut store,
    )
    .await
    .unwrap();
    // Create subscription but it stays Pending (auto_approve=false)
    let sub = SubscriptionId::create(shop.id, None, customer, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Pending);

    let session = TestSession::new(customer);
    let result = shop
        .id
        .register_part(PartId::from(1), &session, &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn register_part_requires_ownership() {
    let mut store = TestStore::prepopulated();
    let shop_owner = UserId::create("Shop", "Owner", &None, &mut store)
        .await
        .unwrap();
    let stranger = UserId::create("Stranger", "User", &None, &mut store)
        .await
        .unwrap();
    let shop = ShopId::create(
        "Bike Barn".into(),
        Some("Best shop".into()),
        true,
        shop_owner,
        &mut store,
    )
    .await
    .unwrap();
    let sub = SubscriptionId::create(shop.id, None, stranger, &mut store)
        .await
        .unwrap();
    assert_eq!(sub.status, SubscriptionStatus::Active);

    let session = TestSession::new(stranger);
    let result = shop
        .id
        .register_part(PartId::from(1), &session, &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

#[tokio::test]
async fn register_part_attached_rejected() {
    let (mut store, _, customer, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::new(customer);
    // Front Wheel A (PartId::from(2)) is attached to Bike A, so it can't be registered
    let result = shop
        .id
        .register_part(PartId::from(2), &session, &mut store)
        .await;
    assert!(matches!(result, Err(Error::BadRequest(_))));
}

#[tokio::test]
async fn unregister_part_by_owner() {
    let (mut store, _, customer, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::new(customer);
    shop.id
        .register_part(PartId::from(1), &session, &mut store)
        .await
        .unwrap();

    let summary = shop
        .id
        .unregister_part(PartId::from(1), &session, &mut store)
        .await
        .unwrap();
    // Bike A + Front Wheel A + Rear Wheel A + Chain A + both tires = 6
    assert_eq!(summary.parts.len(), 6);
    for part in summary.parts.live() {
        assert_eq!(part.shop, None, "part {} should have no shop", part.id);
    }
}

#[tokio::test]
async fn unregister_part_by_shop_owner() {
    let (mut store, shop_owner, customer, shop) =
        setup_shop_and_sub(TestStore::prepopulated()).await;
    let cust_session = TestSession::new(customer);
    shop.id
        .register_part(PartId::from(1), &cust_session, &mut store)
        .await
        .unwrap();

    let shop_session = TestSession::with_shop(shop_owner, shop.id);
    let summary = shop
        .id
        .unregister_part(PartId::from(1), &shop_session, &mut store)
        .await
        .unwrap();
    // Bike A + Front Wheel A + Rear Wheel A + Chain A + both tires = 6
    assert_eq!(summary.parts.len(), 6);
    for part in summary.parts.live() {
        assert_eq!(part.shop, None, "part {} should have no shop", part.id);
    }
}

#[tokio::test]
async fn get_parts_returns_registered() {
    let (mut store, shop_owner, customer, shop) =
        setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::new(customer);
    shop.id
        .register_part(PartId::from(1), &session, &mut store)
        .await
        .unwrap();

    let parts = shop.id.get_parts(shop_owner, &mut store).await.unwrap();
    assert!(!parts.is_empty());
    for part in &parts {
        assert_eq!(
            part.shop,
            Some(shop.id),
            "part {} should have shop",
            part.id
        );
    }
}

#[tokio::test]
async fn get_parts_forbidden_non_owner() {
    let (mut store, _, _, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let stranger = UserId::create("Stranger", "User", &None, &mut store)
        .await
        .unwrap();
    let result = shop.id.get_parts(stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}

// === Shop Maintenance / Ownership Transfer (Tier C) ===

#[tokio::test]
async fn shop_creates_part_in_shop_context() {
    let (mut store, shop_owner, _, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::with_shop(shop_owner, shop.id);
    let part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        OffsetDateTime::now_utc(),
        &session,
        &mut store,
    )
    .await
    .unwrap();
    assert_eq!(part.owner, shop_owner);
    assert_eq!(part.shop, Some(shop.id));
}

#[tokio::test]
async fn shop_owner_attaches_to_customer_bike() {
    let (mut store, shop_owner, _, shop) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let session = TestSession::with_shop(shop_owner, shop.id);

    let new_part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        OffsetDateTime::now_utc(),
        &session,
        &mut store,
    )
    .await
    .unwrap();

    let hook = CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN);
    attach_assembly(
        &session,
        new_part.id,
        OffsetDateTime::now_utc(),
        PartId::from(1),
        hook,
        false,
        &mut store,
    )
    .await
    .unwrap();

    // New chain should be attached to Bike A
    let now = OffsetDateTime::now_utc();
    assert!(
        is_attached(new_part.id, now, &mut store).await.unwrap(),
        "new chain should be attached"
    );
}

#[tokio::test]
async fn attach_transfers_ownership_to_customer() {
    let (mut store, shop_owner, customer, shop) =
        setup_shop_and_sub(TestStore::prepopulated()).await;
    let cust_session = TestSession::new(customer);
    shop.id
        .register_part(PartId::from(1), &cust_session, &mut store)
        .await
        .unwrap();

    let shop_session = TestSession::with_shop(shop_owner, shop.id);
    let new_part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        OffsetDateTime::now_utc(),
        &shop_session,
        &mut store,
    )
    .await
    .unwrap();
    assert_eq!(new_part.owner, shop_owner);

    let hook = CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN);
    attach_assembly(
        &shop_session,
        new_part.id,
        OffsetDateTime::now_utc(),
        PartId::from(1),
        hook,
        false,
        &mut store,
    )
    .await
    .unwrap();

    let updated = new_part.id.read(&mut store).await.unwrap();
    assert_eq!(updated.owner, customer);
    assert_eq!(updated.shop, Some(shop.id));
}

#[tokio::test]
async fn shop_owner_detaches_from_customer_bike() {
    let (mut store, shop_owner, customer, shop) =
        setup_shop_and_sub(TestStore::prepopulated()).await;
    let cust_session = TestSession::new(customer);
    shop.id
        .register_part(PartId::from(1), &cust_session, &mut store)
        .await
        .unwrap();

    let session = TestSession::with_shop(shop_owner, shop.id);
    detach_assembly(
        &session,
        PartId::from(4),
        OffsetDateTime::now_utc(),
        false,
        &mut store,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn no_shop_cannot_access_foreign_bike() {
    let (mut store, _, _, _) = setup_shop_and_sub(TestStore::prepopulated()).await;
    let stranger = UserId::create("Stranger", "User", &None, &mut store)
        .await
        .unwrap();
    let session = TestSession::new(stranger);

    let result = PartId::from(1).part(&session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
}
