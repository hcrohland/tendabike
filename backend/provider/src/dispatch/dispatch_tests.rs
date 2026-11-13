//! The dispatch suite (ADR-0014): the `login` and `accept_event` dispatch
//! functions and their match arms, against the store mocks.
//!
//! The dispatch functions are bounded on the full store for both providers
//! (`StravaStore + IcuStore`), so the suite's store double composes the three
//! provider-layer mocks — the domain's [`MockStore`] plus the two providers'
//! data-store automocks — delegating field by field. The arms' work runs for
//! real against the canned store outcomes; the login's token exchange is the
//! one piece that needs the live provider API, so the login tests cover the
//! arms' scope check (which precedes it).

use std::sync::{Arc, Mutex, Once};

use tb_domain::test_support::MockStore;
use tb_domain::{
    ActTypeId, Activity, ActivityId, Attachment, Notification, NotificationId, NotificationKind,
    NotificationLifecycle, NotificationSubject, OnboardingStatus, Part, PartId, PartNote,
    PartNoteId, PartTypeId, Service, ServiceId, ServicePlan, ServicePlanId, Shop, ShopId,
    ShopSubscription, Store, SubscriptionId, SubscriptionStatus, TbResult, Usage, UsageId, User,
    UserId,
};
use time::OffsetDateTime;

use crate::Provider;
use crate::icu::test_support::MockIcuDataStore;
use crate::icu::{IcuDataStore, IcuEvent, IcuId, IcuStore, IcuUser};
use crate::strava::event::Event;
use crate::strava::test_support::MockStravaDataStore;
use crate::strava::{StravaDataStore, StravaId, StravaStore, StravaUser};

use super::{accept_event, login};

// ─── The composite store double ─────────────────────────────────────────────
//
// The dispatch's `StravaStore + IcuStore` bound: the domain's [`MockStore`]
// (the full `Store` seam) plus the two providers' data stores, delegated
// field by field. The mocks carry the canned outcomes the tests assert on.

/// The dispatch suite's store double: [`MockStore`] (the domain) plus
/// [`MockStravaDataStore`] and [`MockIcuDataStore`], delegated field by field
/// — the `StravaStore + IcuStore` bound the dispatch functions carry.
#[derive(Default)]
struct DispatchStore {
    strava: MockStravaDataStore,
    icu: MockIcuDataStore,
    domain: MockStore,
}

#[async_trait::async_trait]
impl StravaDataStore for DispatchStore {
    async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        self.strava.stravauser_get_all().await
    }
    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        self.strava.stravauser_get_by_tbid(id).await
    }
    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        self.strava.stravauser_get_by_stravaid(id).await
    }
    async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        self.strava.stravauser_create(user).await
    }
    async fn stravauser_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<String>,
    ) -> TbResult<StravaUser> {
        self.strava.stravauser_update_token(stravaid, refresh).await
    }
    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.strava.stravauser_delete(user).await
    }
    async fn strava_event_create(&mut self, e: Event) -> TbResult<()> {
        self.strava.strava_event_create(e).await
    }
    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.strava.strava_event_delete(event_id).await
    }
    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        self.strava.strava_event_set_time(e_id, e_time).await
    }
    async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>> {
        self.strava.strava_event_get_next_for_user(user).await
    }
    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        self.strava.strava_event_get_later(obj_id, oid).await
    }
    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.strava.strava_events_delete_batch(values).await
    }
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        self.strava.strava_events_get_count_for_user(user).await
    }
    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        self.strava.strava_events_delete_for_user(user).await
    }
}

#[async_trait::async_trait]
impl IcuDataStore for DispatchStore {
    async fn icuser_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        self.icu.icuser_get_all().await
    }
    async fn icuser_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
        self.icu.icuser_get_by_tbid(id).await
    }
    async fn icuser_get_by_icuid(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
        self.icu.icuser_get_by_icuid(id).await
    }
    async fn icuser_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        self.icu.icuser_create(user).await
    }
    async fn icuser_update_token(
        &mut self,
        icuid: IcuId,
        access_token: Option<String>,
    ) -> TbResult<IcuUser> {
        self.icu.icuser_update_token(icuid, access_token).await
    }
    async fn icuser_delete(&mut self, user: UserId) -> TbResult<usize> {
        self.icu.icuser_delete(user).await
    }
    async fn icu_event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        self.icu.icu_event_create(e).await
    }
    async fn icu_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.icu.icu_event_delete(event_id).await
    }
    async fn icu_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        self.icu.icu_event_set_time(e_id, e_time).await
    }
    async fn icu_event_get_next_for_user(&mut self, user: IcuId) -> TbResult<Option<IcuEvent>> {
        self.icu.icu_event_get_next_for_user(user).await
    }
    async fn icu_events_get_count_for_user(&mut self, user: &IcuId) -> TbResult<i64> {
        self.icu.icu_events_get_count_for_user(user).await
    }
    async fn icu_events_get_latest_time(&mut self, user: &IcuId) -> TbResult<Option<i64>> {
        self.icu.icu_events_get_latest_time(user).await
    }
    async fn icu_events_delete_for_user(&mut self, user: &IcuId) -> TbResult<usize> {
        self.icu.icu_events_delete_for_user(user).await
    }
}

#[async_trait::async_trait]
impl tb_domain::PartStore for DispatchStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        self.domain.partid_get_part(pid).await
    }
    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        self.domain.part_get_all_for_userid(uid).await
    }
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
    ) -> TbResult<Part> {
        self.domain
            .part_create(
                what, name, vendor, model, purchase, source, usage, owner, shop,
            )
            .await
    }
    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        self.domain.part_update(part).await
    }
    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        self.domain.part_delete(part).await
    }
    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        self.domain.parts_delete(parts).await
    }
    async fn partid_get_by_source(&mut self, source: &str) -> TbResult<Option<PartId>> {
        self.domain.partid_get_by_source(source).await
    }
    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_id: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        self.domain.parts_register_shop(shop_id, part_id).await
    }
    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        self.domain.parts_unregister_shop(part_ids).await
    }
    async fn part_get_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        self.domain.part_get_for_shop(shop_id).await
    }
}

#[async_trait::async_trait]
impl tb_domain::UserStore for DispatchStore {
    async fn user_get(&mut self, uid: UserId) -> TbResult<User> {
        self.domain.user_get(uid).await
    }
    async fn user_create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        provider: &str,
    ) -> TbResult<User> {
        self.domain
            .user_create(firstname, lastname, avatar, provider)
            .await
    }
    async fn user_update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        self.domain
            .user_update(uid, firstname, lastname, avatar)
            .await
    }
    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        self.domain.user_delete(user).await
    }
    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User> {
        self.domain.update_onboarding_status(uid, status).await
    }
}

#[async_trait::async_trait]
impl tb_domain::ShopStore for DispatchStore {
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        owner: UserId,
    ) -> TbResult<Shop> {
        self.domain
            .shop_create(name, description, auto_approve, owner)
            .await
    }
    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> {
        self.domain.shop_get(id).await
    }
    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    ) -> TbResult<Shop> {
        self.domain
            .shop_update(id, name, description, auto_approve)
            .await
    }
    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> {
        self.domain.shop_delete(id).await
    }
    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> {
        self.domain.shops_get_all_for_user(user_id).await
    }
    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>> {
        self.domain.shops_search(query).await
    }
    async fn subscription_create(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<ShopSubscription> {
        self.domain.subscription_create(shop_id, user_id).await
    }
    async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription> {
        self.domain.subscription_get(id).await
    }
    async fn subscription_find_active(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        self.domain.subscription_find_active(shop_id, user_id).await
    }
    async fn subscription_find_pending(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        self.domain
            .subscription_find_pending(shop_id, user_id)
            .await
    }
    async fn subscription_update_status(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        self.domain.subscription_update_status(id, status).await
    }
    async fn subscription_approve(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        self.domain.subscription_approve(id, status).await
    }
    async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()> {
        self.domain.subscription_delete(id).await
    }
    async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>> {
        self.domain.subscriptions_for_shop(shop_id).await
    }
    async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>> {
        self.domain.subscriptions_for_user(user_id).await
    }
}

#[async_trait::async_trait]
impl tb_domain::ActivityStore for DispatchStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        self.domain.activity_create(act).await
    }
    async fn activity_read_by_id(&mut self, aid: &ActivityId) -> TbResult<Option<Activity>> {
        self.domain.activity_read_by_id(aid).await
    }
    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        self.domain.activity_update(act).await
    }
    async fn activity_delete(&mut self, aid: &ActivityId) -> TbResult<usize> {
        self.domain.activity_delete(aid).await
    }
    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        self.domain.activities_delete(activities).await
    }
    async fn activity_get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        self.domain.activity_get_all(uid).await
    }
    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        self.domain
            .activities_find_by_gear_and_time(part, begin, end)
            .await
    }
    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        self.domain.get_by_user_and_time(uid, rstart).await
    }
    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        self.domain
            .activity_set_gear_if_null(user, types, partid)
            .await
    }
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        self.domain.activity_get_really_all().await
    }
}

#[async_trait::async_trait]
impl tb_domain::AttachmentStore for DispatchStore {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        self.domain.attachment_create(att).await
    }
    async fn attachment_delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        self.domain.attachment_delete(att).await
    }
    async fn attachments_delete_by_parts(&mut self, parts: &[Part]) -> TbResult<usize> {
        self.domain.attachments_delete_by_parts(parts).await
    }
    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        self.domain
            .attachment_get_by_gear_and_time(act_gear, start)
            .await
    }
    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        self.domain.attachments_all_by_part(id).await
    }
    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        self.domain.attachment_get_by_part_and_time(pid, time).await
    }
    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<PartTypeId>,
        gear: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        self.domain
            .assembly_get_by_types_time_and_gear(types, gear, time)
            .await
    }
    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what: PartTypeId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        self.domain
            .attachment_find_part_of_type_at_hook_and_time(what, gear, hook, time)
            .await
    }
    async fn attachment_find_successor(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
        what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        self.domain
            .attachment_find_successor(part_id, gear, hook, time, what)
            .await
    }
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        self.domain
            .attachment_find_later_attachment_for_part(part_id, time)
            .await
    }
    async fn attachment_find_part_attached_already(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        self.domain
            .attachment_find_part_attached_already(part_id, gear, hook, time)
            .await
    }
}

#[async_trait::async_trait]
impl tb_domain::PartNoteStore for DispatchStore {
    async fn partnote_create_text(
        &mut self,
        part: PartId,
        name: String,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        self.domain.partnote_create_text(part, name, created).await
    }
    async fn partnote_create_file(
        &mut self,
        part: PartId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        self.domain
            .partnote_create_file(part, name, mime, filename, size, data, created)
            .await
    }
    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>> {
        self.domain.partnote_all_by_part(part).await
    }
    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        self.domain.partnote_get(id).await
    }
    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>> {
        self.domain.partnote_file(id).await
    }
    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote> {
        self.domain.partnote_update_text(id, name).await
    }
    async fn partnote_update_file(
        &mut self,
        id: PartNoteId,
        name: String,
        mime: String,
        filename: Option<String>,
        size: i64,
        data: Vec<u8>,
    ) -> TbResult<PartNote> {
        self.domain
            .partnote_update_file(id, name, mime, filename, size, data)
            .await
    }
    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        self.domain.partnote_remove_file(id).await
    }
    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId> {
        self.domain.partnote_delete(id).await
    }
}

#[async_trait::async_trait]
impl tb_domain::UsageStore for DispatchStore {
    async fn usage_get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> {
        self.domain.usage_get(uid).await
    }
    async fn usages_update(&mut self, usage: &[Usage]) -> TbResult<usize> {
        self.domain.usages_update(usage).await
    }
    async fn usage_delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        self.domain.usage_delete(usage).await
    }
    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize> {
        self.domain.usages_delete(usages).await
    }
    async fn usage_delete_all(&mut self) -> TbResult<usize> {
        self.domain.usage_delete_all().await
    }
}

#[async_trait::async_trait]
impl tb_domain::ServiceStore for DispatchStore {
    async fn service_create(&mut self, service: Service) -> TbResult<Service> {
        self.domain.service_create(service).await
    }
    async fn service_get(&mut self, service: ServiceId) -> TbResult<Service> {
        self.domain.service_get(service).await
    }
    async fn service_update(&mut self, service: Service) -> TbResult<Service> {
        self.domain.service_update(service).await
    }
    async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize> {
        self.domain.service_delete(service).await
    }
    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> {
        self.domain.services_delete(services).await
    }
    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        self.domain.services_by_part(part).await
    }
}

#[async_trait::async_trait]
impl tb_domain::ServicePlanStore for DispatchStore {
    async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        self.domain.serviceplan_create(plan).await
    }
    async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        self.domain.serviceplan_get(plan).await
    }
    async fn serviceplan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        self.domain.serviceplan_update(plan).await
    }
    async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        self.domain.serviceplan_delete(plan).await
    }
    async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize> {
        self.domain.serviceplans_delete(serviceplans).await
    }
    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> {
        self.domain.by_part(part).await
    }
    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> {
        self.domain.by_user(uid).await
    }
}

#[async_trait::async_trait]
impl tb_domain::NotificationStore for DispatchStore {
    async fn notification_create(
        &mut self,
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
    ) -> TbResult<Notification> {
        self.domain
            .notification_create(recipient, lifecycle, kind, subject, message)
            .await
    }
    async fn notification_read(
        &mut self,
        id: NotificationId,
        recipient: UserId,
    ) -> TbResult<Option<Notification>> {
        self.domain.notification_read(id, recipient).await
    }
    async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>> {
        self.domain.notification_read_all(recipient).await
    }
    async fn notification_all_for_user(
        &mut self,
        recipient: UserId,
    ) -> TbResult<Vec<Notification>> {
        self.domain.notification_all_for_user(recipient).await
    }
    async fn notification_delete(
        &mut self,
        id: NotificationId,
        recipient: UserId,
    ) -> TbResult<Option<NotificationId>> {
        self.domain.notification_delete(id, recipient).await
    }
}

impl Store for DispatchStore {}
impl StravaStore for DispatchStore {}
impl IcuStore for DispatchStore {}

// ─── Fixtures ───────────────────────────────────────────────────────────────

/// A raw Strava webhook body: an activity-create event for owner `owner`.
fn strava_activity_body(owner: i32) -> String {
    format!(
        r#"{{"object_type":"activity","object_id":11,"aspect_type":"create","updates":{{}},"owner_id":{owner},"subscription_id":1,"event_time":1767348000}}"#
    )
}

/// A raw Strava webhook body: an athlete deauthorization for owner `owner`.
fn strava_deauth_body(owner: i32) -> String {
    format!(
        r#"{{"object_type":"athlete","object_id":{owner},"aspect_type":"update","updates":{{"authorized":"false"}},"owner_id":{owner},"subscription_id":1,"event_time":1767348000}}"#
    )
}

/// A raw intervals.icu webhook body with the given `secret` and events JSON
/// array elements.
fn icu_body(secret: &str, events: &str) -> String {
    format!(r#"{{"secret":"{secret}","events":[{events}]}}"#)
}

/// One intervals.icu webhook event element.
fn icu_event_json(athlete: &str, typ: &str, activity: &str, timestamp: &str) -> String {
    format!(
        r#"{{"athlete_id":"{athlete}","type":"{typ}","timestamp":"{timestamp}","activity":{{"id":"{activity}"}}}}"#
    )
}

/// The icu webhook secret the intake tests verify against: sets
/// `ICU_WEBHOOK_SECRET` to a known value when it is not already present (a
/// real environment's value takes precedence) and returns the value in force.
fn icu_webhook_secret() -> String {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        if std::env::var("ICU_WEBHOOK_SECRET").is_err() {
            std::env::set_var("ICU_WEBHOOK_SECRET", "icu-test-webhook-secret");
        }
    });
    std::env::var("ICU_WEBHOOK_SECRET").expect("the secret is set by the helper")
}

// ─── accept_event: the Strava arm ───────────────────────────────────────────

/// The Strava arm parses the raw body and queues the event (the `strava_events`
/// row), returning the owner's TendaBike id (the caller's wake).
#[tokio::test]
async fn accept_event_strava_queues_an_activity_event() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let user = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(None));
    let created_cell = created.clone();
    store
        .strava
        .expect_strava_event_create()
        .returning(move |e| {
            *created_cell.lock().expect("the capture is not poisoned") = Some(e.clone());
            Ok(())
        });

    let queued = accept_event(Provider::Strava, &strava_activity_body(42), &mut store).await?;

    assert_eq!(queued, vec![UserId::from(7)]);
    let event = created
        .lock()
        .expect("the capture is not poisoned")
        .clone()
        .expect("the event was queued");
    assert_eq!(event.object_id, 11);
    assert_eq!(event.event_time, 1767348000);
    assert_eq!(event.owner_id, StravaId::from(42));
    Ok(())
}

/// The Strava arm's athlete-deauth branch: the owner is disabled (their queued
/// events dropped, their token cleared), nothing is queued, no wake.
#[tokio::test]
async fn accept_event_strava_athlete_deauth_disables_the_user() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let user = crate::strava::test_support::strava_user(UserId::from(7), 42, true);
    let lookup = user.clone();
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(move |_id| Ok(Some(lookup.clone())));
    store
        .strava
        .expect_strava_events_delete_for_user()
        .returning(|_id| Ok(0));
    store
        .strava
        .expect_stravauser_update_token()
        .returning(move |_id, refresh| {
            assert!(refresh.is_none(), "the deauth clears the token");
            Ok(user.clone())
        });

    let queued = accept_event(Provider::Strava, &strava_deauth_body(42), &mut store).await?;

    assert!(queued.is_empty(), "a deauth queues nothing (no wake)");
    Ok(())
}

/// The Strava arm rejects an unknown owner (the `strava_events` key would have
/// no user) — a `BadRequest`, nothing queued.
#[tokio::test]
async fn accept_event_strava_unknown_owner_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();
    store
        .strava
        .expect_stravauser_get_by_stravaid()
        .returning(|_id| Ok(None));

    let err = accept_event(Provider::Strava, &strava_activity_body(42), &mut store)
        .await
        .expect_err("an unknown owner is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

/// The Strava arm rejects a malformed envelope (the parse failure is a
/// `BadRequest` before any store work).
#[tokio::test]
async fn accept_event_strava_malformed_body_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = accept_event(Provider::Strava, "not a json body", &mut store)
        .await
        .expect_err("a malformed body is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

// ─── accept_event: the icu arm ──────────────────────────────────────────────

/// The icu arm verifies the secret, filters to `ACTIVITY_ANALYZED`, parses the
/// timestamp, and queues one row per analyzed event — returning the users a
/// row was queued for (the caller's wake list).
#[tokio::test]
async fn accept_event_icu_queues_analyzed_events_only() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let user = crate::icu::test_support::icu_user(UserId::from(9), "a1", true);
    store
        .icu
        .expect_icuser_get_by_icuid()
        .returning(move |_id| Ok(Some(user.clone())));
    let created = Arc::new(Mutex::new(Vec::new()));
    let created_cell = created.clone();
    store.icu.expect_icu_event_create().returning(move |e| {
        created_cell
            .lock()
            .expect("the capture is not poisoned")
            .push(e.clone());
        Ok(())
    });

    let body = icu_body(
        &secret,
        &format!(
            "{},{}",
            icu_event_json(
                "a1",
                "ACTIVITY_ANALYZED",
                "act-1",
                "2024-12-06T06:40:47.011+00:00"
            ),
            icu_event_json(
                "a1",
                "ACTIVITY_HEART_RATE",
                "act-2",
                "2024-12-06T06:40:47.011+00:00"
            )
        ),
    );
    let queued = accept_event(Provider::Icu, &body, &mut store).await?;

    // Only the analyzed event was queued (the other type is not subscribed).
    assert_eq!(queued, vec![UserId::from(9)]);
    let created = created.lock().expect("the capture is not poisoned");
    assert_eq!(created.len(), 1);
    let event = &created[0];
    assert_eq!(event.icu_athlete_id, IcuId::from("a1"));
    assert_eq!(event.activity_id, "act-1");
    assert_eq!(event.event_time, 1733467247);
    Ok(())
}

/// The icu arm rejects a bad secret (a `NotAuth`) before any store work.
#[tokio::test]
async fn accept_event_icu_rejects_a_bad_secret() -> TbResult<()> {
    let mut store = DispatchStore::default();
    let body = icu_body(
        "a-different-secret",
        &icu_event_json(
            "a1",
            "ACTIVITY_ANALYZED",
            "act-1",
            "2024-12-06T06:40:47.011+00:00",
        ),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("a bad secret is rejected");
    assert!(matches!(err, tb_domain::Error::NotAuth(_)));
    Ok(())
}

/// The icu arm rejects a malformed envelope (the parse failure is a
/// `BadRequest` before any store work).
#[tokio::test]
async fn accept_event_icu_malformed_body_is_rejected() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = accept_event(Provider::Icu, "not a json body", &mut store)
        .await
        .expect_err("a malformed body is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

/// The icu arm rejects an invalid timestamp (a `BadRequest`) once the secret
/// is verified.
#[tokio::test]
async fn accept_event_icu_rejects_an_invalid_timestamp() -> TbResult<()> {
    let secret = icu_webhook_secret();
    let mut store = DispatchStore::default();
    let body = icu_body(
        &secret,
        &icu_event_json("a1", "ACTIVITY_ANALYZED", "act-1", "not-a-timestamp"),
    );

    let err = accept_event(Provider::Icu, &body, &mut store)
        .await
        .expect_err("an invalid timestamp is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

// ─── login: both arms ───────────────────────────────────────────────────────

/// The login's Strava arm rejects an insufficient scope (a `BadRequest` before
/// the token exchange).
#[tokio::test]
async fn login_strava_rejects_an_insufficient_scope() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = login(Provider::Strava, "code", "read", &mut store)
        .await
        .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}

/// The login's icu arm rejects an insufficient (present) scope (a
/// `BadRequest` before the token exchange).
#[tokio::test]
async fn login_icu_rejects_an_insufficient_scope() -> TbResult<()> {
    let mut store = DispatchStore::default();

    let err = login(Provider::Icu, "code", "WRONG:SCOPE", &mut store)
        .await
        .expect_err("an insufficient scope is rejected");
    assert!(matches!(err, tb_domain::Error::BadRequest(_)));
    Ok(())
}
