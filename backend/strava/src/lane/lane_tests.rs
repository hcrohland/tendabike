//! The `StravaLane` forwarding tests (issue #797): the three provider-specific
//! ops the executor drives — `insert_sync`, `update_gear`, `all_stats` — are
//! thin forwards to the `tb_strava` free functions. The tests run the lane
//! against a canned [`MockLaneStore`] and assert each op routes to its free
//! function with the right arguments (the `auto` → `object_id` mapping, the
//! created-part ids, the per-user stats).

use std::sync::{Arc, Mutex};

use crate::event::ObjectType;
use crate::test_support::{gear_json, part, queue, strava_session, strava_user, user};
use crate::{StravaDataStore, StravaId, StravaStore, StravaUser};
use tb_domain::*;
use tb_provider::ProviderLane;
use time::OffsetDateTime;

// A `StravaStore` double for the lane tests: the full domain `Store` seam
// (mirroring `tb_domain`'s `MockStore`) plus the `StravaDataStore` methods.
// `StravaLane<S>` is generic over `S: StravaStore`, so the lane's forwarding
// is exercised against this canned store. Only the methods each op calls get
// expectations; the rest panic if (unexpectedly) reached.
mockall::mock! {
    pub LaneStore {}

    #[async_trait::async_trait]
    impl PartStore for LaneStore {
        async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part>;
        async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>>;
        async fn part_create(
            &mut self,
            what: PartTypeId,
            name: String,
            vendor: String,
            model: String,
            purchase: OffsetDateTime,
            source: Option<String>,
            usage: UsageId,
            owner: UserId,
            shop: Option<ShopId>,
        ) -> TbResult<Part>;
        async fn part_update(&mut self, part: Part) -> TbResult<Part>;
        async fn part_delete(&mut self, part: PartId) -> TbResult<PartId>;
        async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn partid_get_by_source(&mut self, source: &str) -> TbResult<Option<PartId>>;
        async fn parts_register_shop(
            &mut self,
            shop_id: ShopId,
            part_id: Vec<PartId>,
        ) -> TbResult<Vec<Part>>;
        async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>>;
        async fn part_get_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>>;
    }

    #[async_trait::async_trait]
    impl UserStore for LaneStore {
        async fn user_get(&mut self, uid: UserId) -> TbResult<User>;
        async fn user_create(
            &mut self,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_update(
            &mut self,
            uid: &UserId,
            firstname: &str,
            lastname: &str,
            avatar: &Option<String>,
        ) -> TbResult<User>;
        async fn user_delete(&mut self, user: &UserId) -> TbResult<usize>;
        async fn update_onboarding_status(
            &mut self,
            uid: &UserId,
            status: OnboardingStatus,
        ) -> TbResult<User>;
    }

    #[async_trait::async_trait]
    impl ShopStore for LaneStore {
        async fn shop_create(
            &mut self,
            name: String,
            description: Option<String>,
            auto_approve: bool,
            owner: UserId,
        ) -> TbResult<Shop>;
        async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop>;
        async fn shop_update(
            &mut self,
            id: ShopId,
            name: String,
            description: Option<String>,
            auto_approve: bool,
        ) -> TbResult<Shop>;
        async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize>;
        async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>>;
        async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>>;
        async fn subscription_create(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription>;
        async fn subscription_find_active(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_find_pending(
            &mut self,
            shop_id: ShopId,
            user_id: UserId,
        ) -> TbResult<Option<ShopSubscription>>;
        async fn subscription_update_status(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_approve(
            &mut self,
            id: SubscriptionId,
            status: SubscriptionStatus,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()>;
        async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>>;
        async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>>;
    }

    #[async_trait::async_trait]
    impl ActivityStore for LaneStore {
        async fn activity_create(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_read_by_id(&mut self, aid: &ActivityId) -> TbResult<Option<Activity>>;
        async fn activity_update(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_delete(&mut self, aid: &ActivityId) -> TbResult<usize>;
        async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize>;
        async fn activity_get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>>;
        async fn activities_find_by_gear_and_time(
            &mut self,
            part: PartId,
            begin: OffsetDateTime,
            end: OffsetDateTime,
        ) -> TbResult<Vec<Activity>>;
        async fn get_by_user_and_time(
            &mut self,
            uid: UserId,
            rstart: OffsetDateTime,
        ) -> TbResult<Activity>;
        async fn activity_set_gear_if_null(
            &mut self,
            user: UserId,
            types: Vec<ActTypeId>,
            partid: &PartId,
        ) -> TbResult<Vec<Activity>>;
        async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>>;
    }

    #[async_trait::async_trait]
    impl AttachmentStore for LaneStore {
        async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachment_delete(&mut self, att: Attachment) -> TbResult<Attachment>;
        async fn attachments_delete_by_parts(&mut self, parts: &[Part]) -> TbResult<usize>;
        async fn attachment_get_by_gear_and_time(
            &mut self,
            act_gear: PartId,
            start: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>>;
        async fn attachment_get_by_part_and_time(
            &mut self,
            pid: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn assembly_get_by_types_time_and_gear(
            &mut self,
            types: Vec<PartTypeId>,
            gear: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Vec<Attachment>>;
        async fn attachment_find_part_of_type_at_hook_and_time(
            &mut self,
            what: PartTypeId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_successor(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
            what: PartTypeId,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_later_attachment_for_part(
            &mut self,
            part_id: PartId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
        async fn attachment_find_part_attached_already(
            &mut self,
            part_id: PartId,
            gear: PartId,
            hook: PartTypeId,
            time: OffsetDateTime,
        ) -> TbResult<Option<Attachment>>;
    }

    #[async_trait::async_trait]
    impl PartNoteStore for LaneStore {
        async fn partnote_create_text(
            &mut self,
            part: PartId,
            name: String,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_create_file(
            &mut self,
            part: PartId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
            created: OffsetDateTime,
        ) -> TbResult<PartNote>;
        async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>>;
        async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>>;
        async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote>;
        async fn partnote_update_file(
            &mut self,
            id: PartNoteId,
            name: String,
            mime: String,
            filename: Option<String>,
            size: i64,
            data: Vec<u8>,
        ) -> TbResult<PartNote>;
        async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote>;
        async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId>;
    }

    #[async_trait::async_trait]
    impl UsageStore for LaneStore {
        async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>>;
        async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize>;
        async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage>;
        async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize>;
        async fn usage_delete_all(&mut self) -> TbResult<usize>;
    }

    #[async_trait::async_trait]
    impl ServiceStore for LaneStore {
        async fn service_create(&mut self, service: Service) -> TbResult<Service>;
        async fn service_get(&mut self, service: ServiceId) -> TbResult<Service>;
        async fn service_update(&mut self, service: Service) -> TbResult<Service>;
        async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize>;
        async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize>;
        async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>>;
    }

    #[async_trait::async_trait]
    impl ServicePlanStore for LaneStore {
        async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
        async fn serviceplan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize>;
        async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize>;
        async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
        async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>>;
    }

    #[async_trait::async_trait]
    impl NotificationStore for LaneStore {
        async fn notification_create(
            &mut self,
            recipient: UserId,
            lifecycle: NotificationLifecycle,
            kind: NotificationKind,
            subject: NotificationSubject,
            message: Option<String>,
        ) -> TbResult<Notification>;
        async fn notification_read(
            &mut self,
            id: NotificationId,
            recipient: UserId,
        ) -> TbResult<Option<Notification>>;
        async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
        async fn notification_all_for_user(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
        async fn notification_delete(
            &mut self,
            id: NotificationId,
            recipient: UserId,
        ) -> TbResult<Option<NotificationId>>;
    }

    #[async_trait::async_trait]
    impl StravaDataStore for LaneStore {
        async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>>;
        async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser>;
        async fn stravauser_get_by_stravaid(
            &mut self,
            id: &StravaId,
        ) -> TbResult<Option<StravaUser>>;
        async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser>;
        async fn stravauser_update_token(
            &mut self,
            stravaid: StravaId,
            refresh: Option<String>,
        ) -> TbResult<StravaUser>;
        async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize>;
        async fn strava_event_create(&mut self, e: crate::event::Event) -> TbResult<()>;
        async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;
        async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()>;
        async fn strava_event_get_next_for_user(
            &mut self,
            user: StravaId,
        ) -> TbResult<Option<crate::event::Event>>;
        async fn strava_event_get_later(
            &mut self,
            obj_id: i64,
            oid: StravaId,
        ) -> TbResult<Vec<crate::event::Event>>;
        async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;
        async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64>;
        async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize>;
    }

    impl Store for LaneStore {}
    impl StravaStore for LaneStore {}
}

/// `insert_sync` forwards to the event op: a `true` (migration) sync queues a
/// `Sync` event with `object_id` `1` for the user's Strava id.
#[tokio::test]
async fn lane_insert_sync_queues_the_sync_event() -> TbResult<()> {
    let mut store = MockLaneStore::new();
    let _session = strava_session(UserId::from(1), 42.into());
    let created = Arc::new(Mutex::new(None));
    let rec = created.clone();
    store.expect_strava_event_create().returning(move |e| {
        rec.lock().unwrap().replace(e);
        Ok(())
    });
    let lane = crate::StravaLane::<MockLaneStore>::new();
    lane.insert_sync(UserId::from(1), 100, true, &mut store)
        .await?;
    let event = created.lock().unwrap().take().expect("the sync was queued");
    assert_eq!(event.object_type, ObjectType::Sync);
    assert_eq!(
        event.object_id, 1,
        "auto=true is the onboarding backfill (object_id 1)"
    );
    assert_eq!(event.owner_id, 42.into());
    Ok(())
}

/// A routine (`auto=false`) sync queues a `Sync` event with `object_id` `0`.
#[tokio::test]
async fn lane_insert_sync_routine_sync_has_object_id_zero() -> TbResult<()> {
    let mut store = MockLaneStore::new();
    let _session = strava_session(UserId::from(1), 42.into());
    let created = Arc::new(Mutex::new(None));
    let rec = created.clone();
    store.expect_strava_event_create().returning(move |e| {
        rec.lock().unwrap().replace(e);
        Ok(())
    });
    let lane = crate::StravaLane::<MockLaneStore>::new();
    lane.insert_sync(UserId::from(1), 100, false, &mut store)
        .await?;
    let event = created.lock().unwrap().take().expect("the sync was queued");
    assert_eq!(
        event.object_id, 0,
        "auto=false is a routine sync (object_id 0)"
    );
    Ok(())
}

/// `update_gear` forwards to the gear-hydration op (via the concrete
/// `DomainFacadeImpl`): it imports the user's Strava gear and returns the
/// created part ids.
#[tokio::test]
async fn lane_update_gear_imports_the_parts() -> TbResult<()> {
    let mut store = MockLaneStore::new();
    let _session = strava_session(UserId::from(1), 42.into());
    queue(
        "/athlete",
        r#"{"bikes":[{"id":"b1"}],"shoes":[{"id":"g1"}]}"#,
    );
    queue("/gear/b1", &gear_json("b1", Some(0)));
    queue("/gear/g1", &gear_json("g1", None));
    // No gear exists yet: each lookup is `None`, so each fetch creates a part.
    store.expect_partid_get_by_source().returning(|_s| Ok(None));
    let next_id = Arc::new(std::sync::atomic::AtomicI32::new(0));
    let rec = next_id.clone();
    store.expect_part_create().returning(
        move |_what, _name, _vendor, _model, _purchase, _source, _usage, _owner, _shop| {
            let id = rec.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            Ok(part(PartId::from(id), UserId::from(1), 1.into(), None))
        },
    );
    let lane = crate::StravaLane::<MockLaneStore>::new();
    let parts = lane.update_gear(UserId::from(1), &mut store).await?;
    assert_eq!(parts, vec![PartId::from(1), PartId::from(2)]);
    Ok(())
}

/// `all_stats` forwards to the all-user stat op (via the concrete
/// `DomainFacadeImpl`): every Strava user's stat plus its queued-event count
/// and disabled flag.
#[tokio::test]
async fn lane_all_stats_reads_every_user() -> TbResult<()> {
    let mut store = MockLaneStore::new();
    store.expect_stravauser_get_all().returning(|| {
        Ok(vec![
            strava_user(UserId::from(1), 42, true),
            strava_user(UserId::from(2), 43, false),
        ])
    });
    // The stat is the user record plus the part/activity counts (both empty).
    store.expect_user_get().returning(|uid| Ok(user(uid)));
    store
        .expect_part_get_all_for_userid()
        .returning(|_uid| Ok(Vec::new()));
    store
        .expect_activity_get_all()
        .returning(|_uid| Ok(Vec::new()));
    store
        .expect_strava_events_get_count_for_user()
        .returning(move |u| Ok(if *u == 42.into() { 1 } else { 0 }));
    let lane = crate::StravaLane::<MockLaneStore>::new();
    let stats = lane.all_stats(&mut store).await?;
    assert_eq!(stats.len(), 2);
    let enabled = stats.iter().find(|s| !s.disabled).unwrap();
    assert_eq!(enabled.events, 1);
    let disabled = stats.iter().find(|s| s.disabled).unwrap();
    assert_eq!(disabled.events, 0);
    Ok(())
}
