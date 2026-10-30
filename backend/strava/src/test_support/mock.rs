//! The `mockall` `mock!` store witness and canned-value builders.
//!
//! `#[cfg(test)]` only: `mockall` is a dev-dependency of `tb_strava`, so this
//! module is not available in the `test-support` feature build (which is
//! consumed by `tb_exec`/`tb_sqlx`).

#![allow(clippy::too_many_arguments)]

use mockall::mock;
use time::OffsetDateTime;

use crate::event::Event;
use crate::*;

// A single type accumulating the full `Store` seam (the nine sub-traits plus
// the method-free [`Store`] marker) *and* the Strava surface
// ([`StravaDataStore`] plus the method-free [`StravaStore`] marker) — i.e. the
// `StravaStore` bound the Strava-logic free functions take. No `Txn`: the
// Strava-logic functions run over a bare `&mut impl StravaStore`, not a
// transaction.
//
// `#[async_trait]` on each impl block is the trick that makes the `async fn`
// methods desugar correctly in the generated code (see
// `docs/research/mockall-conn-witness.md` §1). The seam is collision-free
// (post-recipe: renamed methods, `usages_update(&[Usage])`,
// `stravauser_update_token` owning its token), so `mock!`'s flat
// `expect_<name>` API can express every method on one type.
//
// The declared struct name is `StravaConn`, so `mock!` emits
// [`MockStravaConn`]; the impl blocks reference `StravaConn` (the macro
// rewrites it to `MockStravaConn`), and the method-free markers
// `impl Store for StravaConn` / `impl StravaStore for StravaConn` become
// `impl … for MockStravaConn`.
mock! {
    pub StravaConn {}

    #[async_trait::async_trait]
    impl PartStore for StravaConn {
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
        async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>>;
        async fn parts_register_shop(
            &mut self,
            shop_id: ShopId,
            part_id: Vec<PartId>,
        ) -> TbResult<Vec<Part>>;
        async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>>;
        async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>>;
    }

    #[async_trait::async_trait]
    impl UserStore for StravaConn {
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
    impl ShopStore for StravaConn {
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
            message: Option<String>,
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
            response_message: Option<String>,
        ) -> TbResult<ShopSubscription>;
        async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()>;
        async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>>;
        async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>>;
    }

    #[async_trait::async_trait]
    impl ActivityStore for StravaConn {
        async fn activity_create(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>>;
        async fn activity_update(&mut self, act: Activity) -> TbResult<Activity>;
        async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize>;
        async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize>;
        async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>>;
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
    impl AttachmentStore for StravaConn {
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
    impl PartNoteStore for StravaConn {
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
    impl UsageStore for StravaConn {
        async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>>;
        async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize>;
        async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage>;
        async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize>;
        async fn delete_all(&mut self) -> TbResult<usize>;
    }

    #[async_trait::async_trait]
    impl ServiceStore for StravaConn {
        async fn service_create(&mut self, service: Service) -> TbResult<Service>;
        async fn service_get(&mut self, service: ServiceId) -> TbResult<Service>;
        async fn service_update(&mut self, service: Service) -> TbResult<Service>;
        async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize>;
        async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize>;
        async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>>;
    }

    #[async_trait::async_trait]
    impl ServicePlanStore for StravaConn {
        async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan>;
        async fn serviceplan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan>;
        async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize>;
        async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize>;
        async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>>;
        async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>>;
    }

    #[async_trait::async_trait]
    impl NotificationStore for StravaConn {
        async fn notification_create(
            &mut self,
            recipient: UserId,
            lifecycle: NotificationLifecycle,
            kind: NotificationKind,
            subject: NotificationSubject,
            message: Option<String>,
        ) -> TbResult<Notification>;
        async fn notification_read(&mut self, id: NotificationId) -> TbResult<Notification>;
        async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
        async fn notification_all_for_user(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;
    }

    impl Store for StravaConn {}

    #[async_trait::async_trait]
    impl StravaDataStore for StravaConn {
        async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>>;
        async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser>;
        async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>>;
        async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser>;
        async fn stravauser_update_token(
            &mut self,
            stravaid: StravaId,
            refresh: Option<String>,
        ) -> TbResult<StravaUser>;
        async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize>;
        async fn strava_event_create(&mut self, e: Event) -> TbResult<()>;
        async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()>;
        async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()>;
        async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>>;
        async fn strava_event_get_later(
            &mut self,
            obj_id: i64,
            oid: StravaId,
        ) -> TbResult<Vec<Event>>;
        async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()>;
        async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64>;
        async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize>;
    }

    impl StravaStore for StravaConn {}
}

// ─── Canned-value builders for the `mock!` outcomes ─────────────────────────
//
// The Strava-logic tests return canned domain values from the witness's
// `expect_*` hooks; these builders give those values the fields the logic
// reads (ids, owner, type, source) and fill the rest with inert defaults.

/// A canned [`Part`] with the given id, owner, type, and Strava source.
pub fn part(id: PartId, owner: UserId, what: PartTypeId, source: Option<String>) -> Part {
    let now = OffsetDateTime::UNIX_EPOCH;
    Part {
        id,
        owner,
        what,
        name: "Test Part".to_string(),
        vendor: "Test Vendor".to_string(),
        model: "Test Model".to_string(),
        purchase: now,
        last_used: now,
        disposed_at: None,
        usage: UsageId::default(),
        source,
        shop: None,
    }
}

/// A canned [`Activity`] with the given id and owner.
pub fn activity(id: ActivityId, user: UserId) -> Activity {
    Activity {
        id,
        user_id: user,
        what: ActTypeId::from(1),
        name: "Test Ride".to_string(),
        start: OffsetDateTime::UNIX_EPOCH,
        duration: 3600,
        time: None,
        distance: Some(25000),
        climb: Some(300),
        descend: None,
        energy: None,
        gear: None,
        device_name: None,
        external_id: None,
    }
}

/// A canned [`User`] with the given id.
pub fn user(id: UserId) -> User {
    User {
        id,
        name: "Test User".to_string(),
        firstname: "Test".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Completed,
    }
}
