//! The `Shop` entity tests over a canned [`MockStore`] (issue #641).
//!
//! Each test runs the real shop domain logic — the permission matrix
//! (owner/non-owner/read-access/stranger), the register-part cascade and
//! conflict rules, the unregister permission branches, the ownership transfer
//! on attach, and the no-shop foreign-bike guard — over a [`MockStore`] whose
//! methods return canned rows. The domain logic runs for real; only the store
//! I/O is canned. The write→read-back stored-value roundtrip
//! (`shop_update_owner_only`, a `RETURNING *` the sqlx seam lacks) lives in the
//! sqlx seam suite (`sqlx/tests/store_seam.rs`), where the Postgres behavior is
//! the source of truth; the CRUD/query pass-throughs are covered by the sqlx
//! `apiwrite_shop_crud`/`apiread_shop_reads` seam.

use std::sync::{Arc, Mutex};

use super::*;
use crate::test_support::fixtures::sample_purchase_date;
use crate::test_support::part_type_ids;
use crate::test_support::{MockStore, mock_session, mock_session_with_shop};
use part_type_ids::{BIKE, CHAIN, FRONT_WHEEL, REAR_WHEEL};

/// A canned `Notification` row for testing.
fn notification_row(
    recipient: UserId,
    lifecycle: NotificationLifecycle,
    kind: NotificationKind,
    subject: NotificationSubject,
    message: Option<String>,
) -> Notification {
    Notification {
        id: NotificationId::new(),
        recipient,
        lifecycle,
        kind,
        subject,
        message,
        state: NotificationState::New,
        created_at: sample_purchase_date(),
    }
}

// --- Canned rows: the minimal shape the domain logic needs ---

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

fn part_row(id: PartId, owner: UserId, what: PartTypeId, shop: Option<ShopId>) -> Part {
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
        shop,
    }
}

fn sub_row(
    id: SubscriptionId,
    shop: ShopId,
    user: UserId,
    status: SubscriptionStatus,
) -> ShopSubscription {
    let t = sample_purchase_date();
    ShopSubscription {
        id,
        shop_id: shop,
        user_id: user,
        status,
        message: None,
        response_message: None,
        created_at: t,
        updated_at: t,
    }
}

fn attachment_row(
    part: PartId,
    attached: OffsetDateTime,
    gear: PartId,
    hook: PartTypeId,
) -> Attachment {
    Attachment {
        part_id: part,
        attached,
        gear,
        hook,
        detached: MAX_TIME,
        usage: UsageId::new(),
    }
}

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

/// The bike's subpart attachment rows (front wheel, rear wheel, chain, and the
/// two tires mounted on the wheels) — the cascade `subparts` walks.
fn bike_subpart_attachments(bike: PartId) -> Vec<Attachment> {
    vec![
        attachment_row(PartId::from(2), sample_purchase_date(), bike, BIKE),
        attachment_row(PartId::from(3), sample_purchase_date(), bike, BIKE),
        attachment_row(PartId::from(4), sample_purchase_date(), bike, CHAIN),
        attachment_row(
            PartId::from(5),
            sample_purchase_date(),
            PartId::from(2),
            FRONT_WHEEL,
        ),
        attachment_row(
            PartId::from(6),
            sample_purchase_date(),
            PartId::from(3),
            REAR_WHEEL,
        ),
    ]
}

// === Permission matrix (owner / non-owner / read-access / stranger) ===

/// `ShopId::get` passes for the owner.
#[tokio::test]
async fn shop_get_owner_ok() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));

    let id = ShopId::get(shop_id.into(), owner, &mut store).await?;
    assert_eq!(id, shop_id);
    Ok(())
}

/// `ShopId::get` is `Forbidden` for a non-owner.
#[tokio::test]
async fn shop_get_non_owner_forbidden() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let stranger = UserId::from(2);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));

    let result = ShopId::get(shop_id.into(), stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// `get_for_read` passes for the owner.
#[tokio::test]
async fn shop_get_for_read_owner_ok() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));

    let id = ShopId::get_for_read(shop_id.into(), owner, &mut store).await?;
    assert_eq!(id, shop_id);
    Ok(())
}

/// `get_for_read` passes for an active subscriber (not the owner).
#[tokio::test]
async fn shop_get_for_read_active_subscriber_ok() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let subscriber = UserId::from(2);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));
    store
        .expect_subscription_find_active()
        .returning(move |_, _| {
            Ok(Some(sub_row(
                SubscriptionId::from(1),
                shop_id,
                subscriber,
                SubscriptionStatus::Active,
            )))
        });

    let id = ShopId::get_for_read(shop_id.into(), subscriber, &mut store).await?;
    assert_eq!(id, shop_id);
    Ok(())
}

/// `get_for_read` is `Forbidden` for a stranger (neither owner nor subscriber).
#[tokio::test]
async fn shop_get_for_read_stranger_forbidden() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let stranger = UserId::from(2);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));
    store
        .expect_subscription_find_active()
        .returning(|_, _| Ok(None));

    let result = ShopId::get_for_read(shop_id.into(), stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// `ShopId::update` is `Forbidden` for a non-owner (the owner check reads the
/// shop first).
#[tokio::test]
async fn shop_update_non_owner_forbidden() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(1);
    let stranger = UserId::from(2);
    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));

    let result = shop_id
        .update("Hax".into(), None, false, stranger, &mut store)
        .await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Register-part cascade + conflict ===

/// Registering the bike cascades to its subparts: the summary carries the bike
/// plus its five subparts.
#[tokio::test]
async fn register_part_ok() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);
    let bike = PartId::from(1);
    let _ = owner;

    let mut store = MockStore::new();
    // The customer has an active subscription to the shop.
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            customer,
            SubscriptionStatus::Active,
        )])
    });
    // The bike is owned by the customer; checkuser and subparts both read it.
    let bike_row = part_row(bike, customer, BIKE, None);
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));
    // The bike is not attached.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    // The bike's subparts (front wheel, rear wheel, chain, two tires).
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    // Registering sets the shop on the bike and its subparts.
    store
        .expect_parts_register_shop()
        .returning(move |_, parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, Some(shop_id)))
                .collect())
        });
    // The shop (for the notification's recipient).
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", false)));
    // The notification side effect.
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            assert_eq!(recipient, owner);
            assert_eq!(lifecycle, NotificationLifecycle::Transient);
            assert_eq!(kind, NotificationKind::PartRegistered);
            assert_eq!(subject, NotificationSubject::Part(bike));
            assert_eq!(message, None);
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    let session = mock_session(customer);
    let summary = shop_id.register_part(bike, &session, &mut store).await?;
    // Bike A + Front Wheel A + Rear Wheel A + Chain A + both tires = 6.
    assert_eq!(summary.parts.len(), 6);
    // The notification is in the summary.
    assert_eq!(summary.notifications.len(), 1);
    // The shop is the notification's subject: it rides in the summary so the
    // executor routes the frame to the shop's stream (issue #687).
    assert_eq!(summary.shops.len(), 1);
    assert_eq!(summary.shops.get(&shop_id).unwrap().owner, owner);
    Ok(())
}

/// The cascade sets the shop on every registered part.
#[tokio::test]
async fn register_part_cascades_subparts() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let customer = UserId::from(1);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            customer,
            SubscriptionStatus::Active,
        )])
    });
    let bike_row = part_row(bike, customer, BIKE, None);
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    store
        .expect_parts_register_shop()
        .returning(move |_, parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, Some(shop_id)))
                .collect())
        });
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, UserId::from(2), "Bike Barn", false)));
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    let session = mock_session(customer);
    let summary = shop_id.register_part(bike, &session, &mut store).await?;
    for part in summary.parts.live() {
        assert_eq!(
            part.shop,
            Some(shop_id),
            "part {} should have shop",
            part.id
        );
    }
    Ok(())
}

/// Registration requires an active subscription: a stranger with no
/// subscription is `Forbidden`.
#[tokio::test]
async fn register_part_requires_subscription() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let customer = UserId::from(1);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    // No subscription for the customer.
    store
        .expect_subscriptions_for_user()
        .returning(|_| Ok(vec![]));

    let session = mock_session(customer);
    let result = shop_id.register_part(bike, &session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// Registration requires ownership: a subscribed stranger who does not own the
/// bike is `Forbidden`.
#[tokio::test]
async fn register_part_requires_ownership() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let customer = UserId::from(1);
    let stranger = UserId::from(3);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    // The stranger has an active subscription …
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            stranger,
            SubscriptionStatus::Active,
        )])
    });
    // … but the bike is owned by the customer, so checkuser denies.
    let bike_row = part_row(bike, customer, BIKE, None);
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));

    let session = mock_session(stranger);
    let result = shop_id.register_part(bike, &session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

/// An attached part cannot be registered: `BadRequest`.
#[tokio::test]
async fn register_part_attached_rejected() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let customer = UserId::from(1);
    let wheel = PartId::from(2); // Front Wheel A, attached to the bike

    let mut store = MockStore::new();
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            customer,
            SubscriptionStatus::Active,
        )])
    });
    let wheel_row = part_row(wheel, customer, FRONT_WHEEL, None);
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(wheel_row.clone()));
    // The wheel is attached to the bike.
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |_, _| {
            Ok(Some(attachment_row(
                wheel,
                sample_purchase_date(),
                PartId::from(1),
                BIKE,
            )))
        });

    let session = mock_session(customer);
    let result = shop_id.register_part(wheel, &session, &mut store).await;
    assert!(matches!(result, Err(Error::BadRequest(_))));
    Ok(())
}

// === Unregister permission branches ===

/// The part owner can unregister their part: the summary carries the parts with
/// the shop cleared.
#[tokio::test]
async fn unregister_part_by_owner() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let customer = UserId::from(1);
    let shop_owner = UserId::from(2);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    let bike_row = part_row(bike, customer, BIKE, Some(shop_id));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, shop_owner, "Bike Barn", true)));
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    // Unregistering clears the shop on the bike and its subparts.
    store
        .expect_parts_unregister_shop()
        .returning(move |parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, None))
                .collect())
        });
    // The part owner (actor) unregisters: notify the shop owner.
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            assert_eq!(recipient, shop_owner);
            assert_eq!(lifecycle, NotificationLifecycle::Transient);
            assert_eq!(kind, NotificationKind::PartUnregistered);
            assert_eq!(subject, NotificationSubject::Part(bike));
            assert_eq!(message, None);
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    let session = mock_session(customer);
    let summary = shop_id.unregister_part(bike, &session, &mut store).await?;
    assert_eq!(summary.parts.len(), 6);
    for part in summary.parts.live() {
        assert_eq!(part.shop, None, "part {} should have no shop", part.id);
    }
    assert_eq!(summary.notifications.len(), 1);
    // The shop is the notification's subject: it rides in the summary so the
    // executor routes the frame to the shop's stream (issue #687).
    assert_eq!(summary.shops.len(), 1);
    assert_eq!(summary.shops.get(&shop_id).unwrap().owner, shop_owner);
    Ok(())
}

/// The shop owner can unregister a part via the shop session: the checkuser
/// passes through the shop's ownership.
#[tokio::test]
async fn unregister_part_by_shop_owner() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);
    let bike = PartId::from(1);

    let mut store = MockStore::new();
    let bike_row = part_row(bike, customer, BIKE, Some(shop_id));
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));
    // The shop owner acts in the shop: checkuser passes via the shop.
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    store
        .expect_parts_unregister_shop()
        .returning(move |parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, None))
                .collect())
        });
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            assert_eq!(recipient, customer);
            assert_eq!(lifecycle, NotificationLifecycle::Transient);
            assert_eq!(kind, NotificationKind::PartUnregistered);
            assert_eq!(subject, NotificationSubject::Part(bike));
            assert_eq!(message, None);
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    let shop_session = mock_session_with_shop(owner, shop_id);
    let summary = shop_id
        .unregister_part(bike, &shop_session, &mut store)
        .await?;
    assert_eq!(summary.parts.len(), 6);
    for part in summary.parts.live() {
        assert_eq!(part.shop, None, "part {} should have no shop", part.id);
    }
    assert_eq!(summary.notifications.len(), 1);
    Ok(())
}

// === Shop parts (owner only) ===

/// `get_parts` returns the shop's registered parts to the owner.
#[tokio::test]
async fn get_parts_returns_registered() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));
    store.expect_part_get_for_shop().returning(move |_| {
        Ok(vec![
            part_row(PartId::from(1), customer, BIKE, Some(shop_id)),
            part_row(PartId::from(2), customer, FRONT_WHEEL, Some(shop_id)),
        ])
    });

    let parts = shop_id.get_parts(owner, &mut store).await?;
    assert!(!parts.is_empty());
    for part in &parts {
        assert_eq!(
            part.shop,
            Some(shop_id),
            "part {} should have shop",
            part.id
        );
    }
    Ok(())
}

/// `get_parts` is `Forbidden` for a non-owner.
#[tokio::test]
async fn get_parts_forbidden_non_owner() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let stranger = UserId::from(3);

    let mut store = MockStore::new();
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));

    let result = shop_id.get_parts(stranger, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}

// === Ownership transfer on attach ===

/// A part created in a shop context is owned by the shop owner and carries the
/// shop.
#[tokio::test]
async fn shop_creates_part_in_shop_context() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);

    let mut store = MockStore::new();
    store
        .expect_part_create()
        .returning(move |what, _, _, _, _, _, _, uid, shop| {
            Ok(part_row(PartId::from(100), uid, what, shop))
        });

    let session = mock_session_with_shop(owner, shop_id);
    let part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        sample_purchase_date(),
        &session,
        &mut store,
    )
    .await?;
    assert_eq!(part.owner, owner);
    assert_eq!(part.shop, Some(shop_id));
    Ok(())
}

/// A shop owner can attach a part they created to a customer's bike: the part
/// ends up attached (the domain's attach logic runs over the canned rows).
#[tokio::test]
async fn shop_owner_attaches_to_customer_bike() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);
    let bike = PartId::from(1);
    let new_chain = PartId::from(100);
    let hook = CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN);
    let time = sample_purchase_date();

    let mut store = MockStore::new();
    // The new chain (owned by the shop owner) and the customer's bike.
    let new_chain_row = part_row(new_chain, owner, CHAIN, None);
    let bike_row = part_row(bike, customer, BIKE, None);
    store
        .expect_part_create()
        .returning(move |what, _, _, _, _, _, _, uid, shop| {
            Ok(part_row(new_chain, uid, what, shop))
        });
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == new_chain {
            new_chain_row.clone()
        } else {
            bike_row.clone()
        })
    });
    // The shop owner acts in the shop (the gear's access check).
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));
    // The bike is not a sub-assembly (walk-up), the new chain is not attached
    // (detach-self), and the final `is_attached` sees the created row: the same
    // (part, time) is queried twice, so a counter drives the last call.
    let counter = Arc::new(Mutex::new(0));
    let c = counter.clone();
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |part, t| {
            let n = *c.lock().unwrap();
            *c.lock().unwrap() += 1;
            if n == 2 {
                Ok(Some(attachment_row(part, t, bike, hook)))
            } else {
                Ok(None)
            }
        });
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store.expect_part_update().returning(Ok);
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(|_, _, _| Ok(vec![]));
    store
        .expect_usage_get()
        .returning(move |id| Ok(Some(usage_row(id))));
    store.expect_attachment_create().returning(Ok);
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(0));

    let session = mock_session_with_shop(owner, shop_id);
    let part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        time,
        &session,
        &mut store,
    )
    .await?;
    attach_assembly(&session, part.id, time, bike, hook, false, &mut store).await?;

    assert!(
        is_attached(part.id, time, &mut store).await?,
        "new chain should be attached"
    );
    Ok(())
}

/// Attaching a shop-created part to a registered customer bike transfers the
/// part's ownership to the customer (the gear's owner) while keeping the shop.
#[tokio::test]
async fn attach_transfers_ownership_to_customer() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);
    let bike = PartId::from(1);
    let new_chain = PartId::from(100);
    let hook = CHAIN.get().unwrap().hooks.first().copied().unwrap_or(CHAIN);
    let time = sample_purchase_date();

    // The bike is registered to the shop (owner customer, shop set).
    let bike_row = part_row(bike, customer, BIKE, Some(shop_id));
    // Before the attach the new chain is owned by the shop owner; after it, the
    // customer owns it (the transfer) while the shop is retained.
    let new_chain_owner = part_row(new_chain, owner, CHAIN, Some(shop_id));
    let new_chain_customer = part_row(new_chain, customer, CHAIN, Some(shop_id));

    let mut store = MockStore::new();
    // register_part: the customer's active subscription and the bike.
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            customer,
            SubscriptionStatus::Active,
        )])
    });
    // `partid_get_part` is called eight times in order: the bike (register
    // checkuser + subparts, the gear read, the set_owner_and_shop gear read),
    // and the new chain (the part read, the set_owner_and_shop read, the
    // attach's update_timestamps read, and the final read). The final read
    // returns the transferred chain (owned by the customer); a counter drives
    // it.
    let counter = Arc::new(Mutex::new(0));
    let c = counter.clone();
    store.expect_partid_get_part().returning(move |id| {
        let n = *c.lock().unwrap();
        *c.lock().unwrap() += 1;
        if id == bike {
            Ok(bike_row.clone())
        } else if n == 7 {
            Ok(new_chain_customer.clone())
        } else {
            Ok(new_chain_owner.clone())
        }
    });
    store
        .expect_attachment_get_by_part_and_time()
        .returning(|_, _| Ok(None));
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    store
        .expect_parts_register_shop()
        .returning(move |_, parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, Some(shop_id)))
                .collect())
        });
    // Part::create: the new chain, owned by the shop owner, in the shop.
    store
        .expect_part_create()
        .returning(move |what, _, _, _, _, _, _, uid, shop| {
            Ok(part_row(new_chain, uid, what, shop))
        });
    // The shop owner acts in the shop (the gear's access check).
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));
    store
        .expect_attachment_find_part_of_type_at_hook_and_time()
        .returning(|_, _, _, _| Ok(None));
    store.expect_part_update().returning(Ok);
    store
        .expect_attachment_find_successor()
        .returning(|_, _, _, _, _| Ok(None));
    store
        .expect_attachment_find_later_attachment_for_part()
        .returning(|_, _| Ok(None));
    store
        .expect_attachment_find_part_attached_already()
        .returning(|_, _, _, _| Ok(None));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(|_, _, _| Ok(vec![]));
    store
        .expect_usage_get()
        .returning(move |id| Ok(Some(usage_row(id))));
    store.expect_attachment_create().returning(Ok);
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store.expect_usages_update().returning(|_| Ok(0));
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    // Register the bike to the shop (as the customer).
    let cust_session = mock_session(customer);
    shop_id
        .register_part(bike, &cust_session, &mut store)
        .await?;

    // Create the new chain (as the shop owner) and attach it to the bike.
    let shop_session = mock_session_with_shop(owner, shop_id);
    let new_part = Part::create(
        "New Chain".into(),
        "Shimano".into(),
        "CN-M510".into(),
        CHAIN,
        None,
        time,
        &shop_session,
        &mut store,
    )
    .await?;
    assert_eq!(new_part.owner, owner);

    attach_assembly(
        &shop_session,
        new_part.id,
        time,
        bike,
        hook,
        false,
        &mut store,
    )
    .await?;

    // The ownership transferred to the customer; the shop is retained.
    let updated = new_part.id.read(&mut store).await?;
    assert_eq!(updated.owner, customer);
    assert_eq!(updated.shop, Some(shop_id));
    Ok(())
}

/// A shop owner can detach a part from a registered customer bike (the shop
/// session passes the part's access check).
#[tokio::test]
async fn shop_owner_detaches_from_customer_bike() -> TbResult<()> {
    let shop_id = ShopId::from(1);
    let owner = UserId::from(2);
    let customer = UserId::from(1);
    let bike = PartId::from(1);
    let chain = PartId::from(4); // Chain A, attached to the bike
    let time = sample_purchase_date();

    let mut store = MockStore::new();
    // register_part: the customer's active subscription and the bike.
    store.expect_subscriptions_for_user().returning(move |_| {
        Ok(vec![sub_row(
            SubscriptionId::from(1),
            shop_id,
            customer,
            SubscriptionStatus::Active,
        )])
    });
    // The bike (register) and the chain (detach) are both owned by the customer.
    let bike_row = part_row(bike, customer, BIKE, Some(shop_id));
    let chain_row = part_row(chain, customer, CHAIN, Some(shop_id));
    store.expect_partid_get_part().returning(move |id| {
        Ok(if id == chain {
            chain_row.clone()
        } else {
            bike_row.clone()
        })
    });
    // The bike is not attached (register); the chain is attached (detach).
    store
        .expect_attachment_get_by_part_and_time()
        .returning(move |id, _t| {
            if id == chain {
                Ok(Some(attachment_row(
                    chain,
                    sample_purchase_date(),
                    bike,
                    CHAIN,
                )))
            } else {
                Ok(None)
            }
        });
    store
        .expect_assembly_get_by_types_time_and_gear()
        .returning(move |_, _, _| Ok(bike_subpart_attachments(bike)));
    store
        .expect_parts_register_shop()
        .returning(move |_, parts| {
            Ok(parts
                .into_iter()
                .map(|p| part_row(p, customer, BIKE, Some(shop_id)))
                .collect())
        });
    // The shop owner acts in the shop (the chain's access check).
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop_id, owner, "Bike Barn", true)));
    // The detach's usage bookkeeping: the chain's attachment and usage rows,
    // no services, no activities in the window.
    store.expect_attachment_delete().returning(Ok);
    store
        .expect_usage_delete()
        .returning(move |id| Ok(usage_row(id)));
    store.expect_services_by_part().returning(|_| Ok(vec![]));
    store
        .expect_usage_get()
        .returning(move |id| Ok(Some(usage_row(id))));
    store.expect_usages_update().returning(|_| Ok(0));
    store
        .expect_activities_find_by_gear_and_time()
        .returning(|_, _, _| Ok(vec![]));
    store.expect_part_update().returning(Ok);
    store.expect_attachment_create().returning(Ok);
    store.expect_notification_create().returning(
        move |recipient, lifecycle, kind, subject, message| {
            Ok(notification_row(
                recipient, lifecycle, kind, subject, message,
            ))
        },
    );

    // Register the bike to the shop (as the customer).
    let cust_session = mock_session(customer);
    shop_id
        .register_part(bike, &cust_session, &mut store)
        .await?;

    // Detach the chain from the bike (as the shop owner).
    let session = mock_session_with_shop(owner, shop_id);
    detach_assembly(&session, chain, time, false, &mut store).await?;
    Ok(())
}

// === No-shop foreign-bike guard ===

/// A user with no shop session cannot read a part they do not own.
#[tokio::test]
async fn no_shop_cannot_access_foreign_bike() -> TbResult<()> {
    let bike = PartId::from(1);
    let customer = UserId::from(1);
    let stranger = UserId::from(2);

    let mut store = MockStore::new();
    let bike_row = part_row(bike, customer, BIKE, None);
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(bike_row.clone()));

    let session = mock_session(stranger);
    let result = bike.part(&session, &mut store).await;
    assert!(matches!(result, Err(Error::Forbidden(_))));
    Ok(())
}
