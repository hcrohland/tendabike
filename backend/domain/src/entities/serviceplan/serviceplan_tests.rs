use super::*;
use crate::test_support::{MockStore, fixtures, mock_session, mock_session_with_shop};

use fixtures::{sample_purchase_date, test_session, test_user};

fn sample_plan(part: Option<PartId>, name: &str, uid: Option<UserId>) -> ServicePlan {
    ServicePlan {
        id: ServicePlanId::new(),
        part,
        what: PartTypeId::from(1),
        hook: None,
        name: name.to_string(),
        days: Some(30),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid,
        energy: None,
    }
}

/// A canned `Part` row: the minimal shape the domain logic needs (id + owner).
fn part_row(id: PartId, owner: UserId) -> Part {
    Part {
        id,
        owner,
        what: PartTypeId::from(4),
        name: "Test Chain".to_string(),
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

/// A canned `Service` row: the minimal shape the domain logic needs.
fn service_row(id: ServiceId, part: PartId, plans: Vec<ServicePlanId>) -> Service {
    let t = time::macros::datetime!(2024-06-15 10:00 UTC);
    Service {
        id,
        part_id: part,
        time: t,
        redone: t,
        name: "Service".to_string(),
        notes: String::new(),
        usage: UsageId::new(),
        successor: None,
        plans,
    }
}

/// A canned `Shop` row: the minimal shape the domain logic needs (id + owner).
fn shop_row(id: ShopId, owner: UserId) -> Shop {
    Shop {
        id,
        owner,
        name: "Shop 99".to_string(),
        description: None,
        auto_approve: true,
        created_at: sample_purchase_date(),
    }
}

// === Suite 6: ServicePlan — CRUD ===

/// SP-01: ServicePlan create with specific part
#[tokio::test]
async fn service_plan_create_specific_part() -> TbResult<()> {
    let mut store = MockStore::new();
    let part = PartId::from(1);

    let plan = ServicePlan {
        id: ServicePlanId::new(),
        part: Some(part),
        what: PartTypeId::from(2),
        hook: None,
        name: "Service Plan".to_string(),
        days: Some(30),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };
    store.expect_serviceplan_create().returning(Ok);

    let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

    assert_eq!(created.part, Some(part));
    assert_eq!(created.uid, None);
    // create() does NOT auto-set what; it's stored as-is
    assert_eq!(created.what, PartTypeId::from(2));
    Ok(())
}

/// SP-02: ServicePlan create generic plan (no specific part)
#[tokio::test]
async fn service_plan_create_generic_plan() -> TbResult<()> {
    let mut store = MockStore::new();

    let plan = ServicePlan {
        id: ServicePlanId::new(),
        part: None,
        what: PartTypeId::from(3),
        hook: Some(PartTypeId::from(1)),
        name: "Generic Plan".to_string(),
        days: Some(60),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };
    store.expect_serviceplan_create().returning(Ok);

    let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

    assert_eq!(created.part, None);
    assert_eq!(created.uid, Some(test_user()));
    Ok(())
}

/// SP-03: ServicePlan create stores what as-provided (not auto-set from part type)
#[tokio::test]
async fn service_plan_create_sets_what_to_part_type() -> TbResult<()> {
    let mut store = MockStore::new();
    let part = PartId::from(1);

    // create() does NOT auto-set what from part type; it stores as-is
    let plan = ServicePlan {
        id: ServicePlanId::new(),
        part: Some(part),
        what: PartTypeId::from(99),
        hook: None,
        name: "Wrong What".to_string(),
        days: Some(10),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };
    store.expect_serviceplan_create().returning(Ok);

    let created = ServicePlan::create(plan, &test_session(), &mut store).await?;

    assert_eq!(created.what, PartTypeId::from(99));
    Ok(())
}

/// SP-08: ServicePlan update preserves immutable fields
#[tokio::test]
async fn service_plan_update_preserves_immutable_fields() -> TbResult<()> {
    let id = ServicePlanId::new();
    let stored = ServicePlan {
        id,
        part: None,
        what: PartTypeId::from(5),
        hook: Some(PartTypeId::from(7)),
        name: "Original".to_string(),
        days: Some(30),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: Some(test_user()),
        energy: None,
    };

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(stored.clone()));
    store.expect_serviceplan_update().returning(Ok);

    // Update only name; the immutable fields come back from the stored plan.
    let update = ServicePlan {
        id,
        part: None,
        what: PartTypeId::from(99),
        hook: Some(PartTypeId::from(99)),
        name: "Updated".to_string(),
        days: Some(60),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: Some(UserId::from(99)),
        energy: None,
    };
    let result = update.update(&test_session(), &mut store).await?;

    assert_eq!(result.name, "Updated");
    assert_eq!(result.part, None);
    assert_eq!(result.what, PartTypeId::from(5));
    assert_eq!(result.hook, Some(PartTypeId::from(7)));
    assert_eq!(result.uid, Some(test_user()));
    Ok(())
}

/// SP-09: ServicePlan update requires ownership (specific part)
#[tokio::test]
async fn service_plan_update_requires_ownership_specific() -> TbResult<()> {
    let id = ServicePlanId::new();
    let part = PartId::from(1);
    let stored = ServicePlan {
        id,
        part: Some(part),
        what: PartTypeId::from(1),
        hook: None,
        name: "Owned Plan".to_string(),
        days: Some(30),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(stored.clone()));
    // the part is owned by test_user(), so the other user (no shop) is denied
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));

    let other_user = mock_session(UserId::from(99));
    let update = ServicePlan {
        id,
        part: Some(part),
        what: PartTypeId::from(99),
        hook: None,
        name: "Hacked".to_string(),
        days: Some(1),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: None,
        energy: None,
    };
    let result = update.update(&other_user, &mut store).await;
    assert!(result.is_err());

    Ok(())
}

/// SP-10: ServicePlan update requires ownership (generic plan)
#[tokio::test]
async fn service_plan_update_requires_ownership_generic() -> TbResult<()> {
    let id = ServicePlanId::new();
    let stored = ServicePlan {
        id,
        part: None,
        what: PartTypeId::from(1),
        hook: None,
        name: "Generic Owned".to_string(),
        days: Some(30),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: Some(test_user()),
        energy: None,
    };

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(stored.clone()));

    let other_user = mock_session(UserId::from(99));
    let update = ServicePlan {
        id,
        part: None,
        what: PartTypeId::from(99),
        hook: None,
        name: "Hacked".to_string(),
        days: Some(1),
        hours: None,
        km: None,
        climb: None,
        descend: None,
        rides: None,
        uid: Some(UserId::from(99)),
        energy: None,
    };
    let result = update.update(&other_user, &mut store).await;
    assert!(result.is_err());

    Ok(())
}

/// SP-12: deleting a plan unlinks it from the owner's services and returns them
#[tokio::test]
async fn service_plan_delete_unlinks_owner_services() -> TbResult<()> {
    let part = PartId::from(1);
    let created = sample_plan(Some(part), "Unlink Plan", None);
    let plan_id = created.id;
    let service = service_row(ServiceId::new(), part, vec![plan_id]);
    let service_id = service.id;

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(created.clone()));
    // checkuser + owner read both hit partid_get_part; the part is owned by test_user()
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    store
        .expect_part_get_all_for_userid()
        .returning(move |_| Ok(vec![part_row(part, test_user())]));
    store
        .expect_services_by_part()
        .returning(move |_| Ok(vec![service.clone()]));
    store.expect_service_update().returning(Ok);
    store.expect_serviceplan_delete().returning(|_| Ok(1));

    let res = plan_id.delete(&test_session(), &mut store).await?;

    // the response carries the updated service
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, service_id);
    assert!(res[0].plans.is_empty());
    Ok(())
}

/// SP-12a: the unlink is scoped to the deleted plan's owner's services only
#[tokio::test]
async fn service_plan_delete_unlink_scoped_to_owner() -> TbResult<()> {
    let created = sample_plan(None, "Generic Unlink", Some(test_user()));
    let plan_id = created.id;
    let mine = PartId::from(1);
    let my_service = service_row(ServiceId::new(), mine, vec![plan_id]);
    let my_service_id = my_service.id;

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(created.clone()));
    // only the owner's parts are consulted, so any other user's service is never touched
    store
        .expect_part_get_all_for_userid()
        .returning(move |_| Ok(vec![part_row(mine, test_user())]));
    store
        .expect_services_by_part()
        .returning(move |_| Ok(vec![my_service.clone()]));
    store.expect_service_update().returning(Ok);
    store.expect_serviceplan_delete().returning(|_| Ok(1));

    let res = plan_id.delete(&test_session(), &mut store).await?;

    // only the owner's updated service is in the response
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, my_service_id);
    assert!(res[0].plans.is_empty());
    Ok(())
}

/// SP-12b: a shop session deleting a specific-part plan unlinks the part owner's services
#[tokio::test]
async fn service_plan_delete_unlinks_part_owner_via_shop_session() -> TbResult<()> {
    let part = PartId::from(1);
    let shop = ShopId::from(1);
    let created = sample_plan(Some(part), "Shop Delete", None);
    let plan_id = created.id;
    let service = service_row(ServiceId::new(), part, vec![plan_id]);
    let service_id = service.id;

    let shop_session = mock_session_with_shop(UserId::from(99), shop);

    let mut store = MockStore::new();
    store
        .expect_serviceplan_get()
        .returning(move |_| Ok(created.clone()));
    // the part is owned by test_user(); the shop session (owner 99) passes checkuser via the shop
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(part_row(part, test_user())));
    store
        .expect_shop_get()
        .returning(move |_| Ok(shop_row(shop, UserId::from(99))));
    store
        .expect_part_get_all_for_userid()
        .returning(move |_| Ok(vec![part_row(part, test_user())]));
    store
        .expect_services_by_part()
        .returning(move |_| Ok(vec![service.clone()]));
    store.expect_service_update().returning(Ok);
    store.expect_serviceplan_delete().returning(|_| Ok(1));

    let res = plan_id.delete(&shop_session, &mut store).await?;

    // the part owner's updated service is in the response
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, service_id);
    assert!(res[0].plans.is_empty());
    Ok(())
}
