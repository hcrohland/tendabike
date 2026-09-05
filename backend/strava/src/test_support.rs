//! In-memory test doubles for unit tests of the strava crate.
//!
//! `TestStravaStore` wraps the domain `MemStore` and adds in-memory
//! `strava_users` and `events` collections mirroring the SQL semantics.
//! `TestStravaSession` is a scripted `StravaSession` that replays
//! queued JSON responses for exact request URIs.

#![allow(clippy::too_many_arguments)]

use std::collections::{HashMap, VecDeque};

use oauth2::RefreshToken;
use serde::de::DeserializeOwned;
use tb_domain::test_support::MemStore;
use time::OffsetDateTime;

use crate::event::Event;
use crate::*;

/// In-memory [`StravaStore`] delegating domain data to the domain [`MemStore`].
pub struct TestStravaStore {
    pub mem: MemStore,
    pub strava_users: HashMap<UserId, StravaUser>,
    pub events: Vec<Event>,
    next_event_id: i32,
}

impl TestStravaStore {
    pub fn new() -> Self {
        Self {
            mem: MemStore::new(),
            strava_users: HashMap::new(),
            events: Vec::new(),
            next_event_id: 0,
        }
    }

    pub fn insert_user(&mut self, user: StravaUser) {
        self.strava_users.insert(user.tendabike_id, user);
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

#[async_trait::async_trait]
impl StravaStore for TestStravaStore {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        self.strava_users
            .values()
            .find(|u| u.tendabike_id == UserId::from(who))
            .map(|u| i32::from(u.id))
            .ok_or_else(|| Error::NotFound(format!("strava user {who} not found")))
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        self.events.retain(|e| e.id != event_id);
        Ok(())
    }

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        for e in &mut self.events {
            if e.id == e_id {
                e.event_time = e_time;
            }
        }
        Ok(())
    }

    async fn stravaevent_store(&mut self, mut e: Event) -> TbResult<()> {
        self.next_event_id += 1;
        e.id = Some(self.next_event_id);
        self.events.push(e);
        Ok(())
    }

    async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>> {
        Ok(self
            .events
            .iter()
            .filter(|e| e.owner_id == user || e.owner_id == StravaId::default())
            .min_by_key(|e| e.event_time)
            .cloned())
    }

    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        let mut res: Vec<Event> = self
            .events
            .iter()
            .filter(|e| e.object_id == obj_id && e.owner_id == oid)
            .cloned()
            .collect();
        res.sort_by_key(|e| e.event_time);
        Ok(res)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        self.events.retain(|e| !values.contains(&e.id));
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        Ok(self.strava_users.values().cloned().collect())
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        self.strava_users
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("user {id} not registered with strava")))
    }

    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        Ok(self.strava_users.values().find(|u| u.id == *id).cloned())
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        let res = user.clone();
        self.strava_users.insert(user.tendabike_id, user);
        Ok(res)
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        let user = self
            .strava_users
            .values_mut()
            .find(|u| u.id == stravaid)
            .ok_or_else(|| Error::NotFound(format!("strava user {stravaid} not found")))?;
        user.refresh_token = refresh.map(|r| RefreshToken::new(r.clone()));
        Ok(user.clone())
    }

    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        Ok(self.events.iter().filter(|e| &e.owner_id == user).count() as i64)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        let before = self.events.len();
        self.events.retain(|e| &e.owner_id != user);
        Ok(before - self.events.len())
    }

    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        Ok(self.strava_users.remove(&user).is_some() as usize)
    }
}

#[async_trait::async_trait]
impl Store for TestStravaStore {
    async fn commit(self) -> TbResult<()> {
        self.mem.commit().await
    }
}

/// Scripted [`StravaSession`] for tests.
///
/// `request_json` replays queued responses for the exact request URI and
/// records all requested URIs in `requests`.
pub struct TestStravaSession {
    user: UserId,
    strava: StravaId,
    admin: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    pub requests: Vec<String>,
    pub deauthorizes: Vec<StravaId>,
}

impl TestStravaSession {
    pub fn new(user: UserId, strava: StravaId) -> Self {
        Self {
            user,
            strava,
            admin: false,
            responses: HashMap::new(),
            requests: Vec::new(),
            deauthorizes: Vec::new(),
        }
    }

    /// Queue a JSON response body for an exact request URI.
    pub fn queue(&mut self, uri: &str, json: &str) {
        self.responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Ok(json.to_string()));
    }

    /// Queue an error for an exact request URI.
    pub fn queue_error(&mut self, uri: &str, err: Error) {
        self.responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Err(err));
    }
}

impl Session for TestStravaSession {
    fn user_id(&self) -> UserId {
        self.user
    }

    fn shop(&self) -> Option<ShopId> {
        None
    }

    fn set_shop(&mut self, _shop: Option<ShopId>) -> TbResult<()> {
        Ok(())
    }

    fn is_admin(&self) -> bool {
        self.admin
    }
}

#[async_trait::async_trait]
impl StravaSession for TestStravaSession {
    fn strava_id(&self) -> StravaId {
        self.strava
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaStore,
    ) -> TbResult<T> {
        self.requests.push(uri.to_string());
        let body = self
            .responses
            .get_mut(uri)
            .and_then(|q| q.pop_front())
            .unwrap_or(Err(Error::BadRequest(format!(
                "unexpected strava request: {uri}"
            ))));
        let body = body?;
        serde_json::from_str(&body).map_err(|e| Error::AnyFailure(e.into()))
    }

    async fn deauthorize(&mut self, _store: &mut impl StravaStore) -> TbResult<()> {
        self.deauthorizes.push(self.strava);
        Ok(())
    }
}

/// Builds a [`StravaUser`] for tests.
pub fn strava_user(tbid: UserId, stravaid: i32, enabled: bool) -> StravaUser {
    StravaUser {
        id: stravaid.into(),
        tendabike_id: tbid,
        refresh_token: enabled.then(|| RefreshToken::new("test-token".to_string())),
    }
}

/// Minimal Strava activity JSON with the given id, type, and gear id.
pub fn activity_json(id: i64, typ: &str, gear: Option<&str>) -> String {
    let gear = match gear {
        Some(g) => format!("\"{g}\""),
        None => "null".to_string(),
    };
    format!(
        r#"{{"id":{id},"type":"{typ}","name":"Test Ride","start_date":"2026-01-02T10:00:00Z","utc_offset":0,"elapsed_time":3600,"moving_time":3300,"distance":25000.0,"total_elevation_gain":300.0,"kilojoules":4000.0,"gear_id":{gear},"device_name":"Test Device","external_id":null}}"#
    )
}

/// Minimal Strava gear JSON with the given id; `frame_type` None means shoes.
pub fn gear_json(id: &str, frame_type: Option<i32>) -> String {
    let ft = frame_type
        .map(|t| t.to_string())
        .unwrap_or_else(|| "null".into());
    format!(
        r#"{{"id":"{id}","name":"Test Bike","brand_name":"Test Brand","model_name":"Test Model","frame_type":{ft}}}"#
    )
}

#[async_trait::async_trait]
impl PartStore for TestStravaStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
        PartStore::partid_get_part(&mut self.mem, pid).await
    }
    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> {
        PartStore::part_get_all_for_userid(&mut self.mem, uid).await
    }
    async fn part_create(
        &mut self,
        what: PartTypeId,
        name: String,
        vendor: String,
        model: String,
        purchase: OffsetDateTime,
        source: Option<String>,
        notes: String,
        usage: UsageId,
        owner: UserId,
        shop: Option<ShopId>,
    ) -> TbResult<Part> {
        PartStore::part_create(
            &mut self.mem,
            what,
            name,
            vendor,
            model,
            purchase,
            source,
            notes,
            usage,
            owner,
            shop,
        )
        .await
    }
    async fn part_update(&mut self, part: Part) -> TbResult<Part> {
        PartStore::part_update(&mut self.mem, part).await
    }
    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> {
        PartStore::part_delete(&mut self.mem, part).await
    }
    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> {
        PartStore::parts_delete(&mut self.mem, parts).await
    }
    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> {
        PartStore::partid_get_by_source(&mut self.mem, strava_id).await
    }
    async fn parts_register_shop(
        &mut self,
        shop_id: ShopId,
        part_id: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        PartStore::parts_register_shop(&mut self.mem, shop_id, part_id).await
    }
    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        PartStore::parts_unregister_shop(&mut self.mem, part_ids).await
    }
    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> {
        PartStore::shop_get_parts(&mut self.mem, shop_id).await
    }
}

#[async_trait::async_trait]
impl UserStore for TestStravaStore {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        UserStore::get(&mut self.mem, uid).await
    }
    async fn create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        UserStore::create(&mut self.mem, firstname, lastname, avatar).await
    }
    async fn update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        UserStore::update(&mut self.mem, uid, firstname, lastname, avatar).await
    }
    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        UserStore::user_delete(&mut self.mem, user).await
    }
    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User> {
        UserStore::update_onboarding_status(&mut self.mem, uid, status).await
    }
}

#[async_trait::async_trait]
impl ShopStore for TestStravaStore {
    async fn shop_create(
        &mut self,
        name: String,
        description: Option<String>,
        auto_approve: bool,
        owner: UserId,
    ) -> TbResult<Shop> {
        ShopStore::shop_create(&mut self.mem, name, description, auto_approve, owner).await
    }
    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> {
        ShopStore::shop_get(&mut self.mem, id).await
    }
    async fn shop_update(
        &mut self,
        id: ShopId,
        name: String,
        description: Option<String>,
        auto_approve: bool,
    ) -> TbResult<Shop> {
        ShopStore::shop_update(&mut self.mem, id, name, description, auto_approve).await
    }
    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> {
        ShopStore::shop_delete(&mut self.mem, id).await
    }
    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> {
        ShopStore::shops_get_all_for_user(&mut self.mem, user_id).await
    }
    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>> {
        ShopStore::shops_search(&mut self.mem, query).await
    }
    async fn subscription_create(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
        message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        ShopStore::subscription_create(&mut self.mem, shop_id, user_id, message).await
    }
    async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription> {
        ShopStore::subscription_get(&mut self.mem, id).await
    }
    async fn subscription_find_active(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        ShopStore::subscription_find_active(&mut self.mem, shop_id, user_id).await
    }
    async fn subscription_find_pending(
        &mut self,
        shop_id: ShopId,
        user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        ShopStore::subscription_find_pending(&mut self.mem, shop_id, user_id).await
    }
    async fn subscription_update_status(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        ShopStore::subscription_update_status(&mut self.mem, id, status).await
    }
    async fn subscription_approve(
        &mut self,
        id: SubscriptionId,
        status: SubscriptionStatus,
        response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        ShopStore::subscription_approve(&mut self.mem, id, status, response_message).await
    }
    async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()> {
        ShopStore::subscription_delete(&mut self.mem, id).await
    }
    async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>> {
        ShopStore::subscriptions_for_shop(&mut self.mem, shop_id).await
    }
    async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>> {
        ShopStore::subscriptions_for_user(&mut self.mem, user_id).await
    }
}

#[async_trait::async_trait]
impl ActivityStore for TestStravaStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        ActivityStore::activity_create(&mut self.mem, act).await
    }
    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        ActivityStore::activity_read_by_id(&mut self.mem, aid).await
    }
    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        ActivityStore::activity_update(&mut self.mem, act).await
    }
    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        ActivityStore::activity_delete(&mut self.mem, aid).await
    }
    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        ActivityStore::activities_delete(&mut self.mem, activities).await
    }
    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        ActivityStore::get_all(&mut self.mem, uid).await
    }
    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        ActivityStore::activities_find_by_gear_and_time(&mut self.mem, part, begin, end).await
    }
    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        ActivityStore::get_by_user_and_time(&mut self.mem, uid, rstart).await
    }
    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        ActivityStore::activity_set_gear_if_null(&mut self.mem, user, types, partid).await
    }
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        ActivityStore::activity_get_really_all(&mut self.mem).await
    }
}

#[async_trait::async_trait]
impl AttachmentStore for TestStravaStore {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> {
        AttachmentStore::attachment_create(&mut self.mem, att).await
    }
    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> {
        AttachmentStore::delete(&mut self.mem, att).await
    }
    async fn attachments_delete_by_parts(&mut self, parts: &[crate::Part]) -> TbResult<usize> {
        AttachmentStore::attachments_delete_by_parts(&mut self.mem, parts).await
    }
    async fn attachment_get_by_gear_and_time(
        &mut self,
        act_gear: PartId,
        start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        AttachmentStore::attachment_get_by_gear_and_time(&mut self.mem, act_gear, start).await
    }
    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> {
        AttachmentStore::attachments_all_by_part(&mut self.mem, id).await
    }
    async fn attachment_get_by_part_and_time(
        &mut self,
        pid: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        AttachmentStore::attachment_get_by_part_and_time(&mut self.mem, pid, time).await
    }
    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        types: Vec<crate::PartTypeId>,
        gear: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        AttachmentStore::assembly_get_by_types_time_and_gear(&mut self.mem, types, gear, time).await
    }
    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        what: PartTypeId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        AttachmentStore::attachment_find_part_of_type_at_hook_and_time(
            &mut self.mem,
            what,
            gear,
            hook,
            time,
        )
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
        AttachmentStore::attachment_find_successor(&mut self.mem, part_id, gear, hook, time, what)
            .await
    }
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        part_id: PartId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        AttachmentStore::attachment_find_later_attachment_for_part(&mut self.mem, part_id, time)
            .await
    }
    async fn attachment_find_part_attached_already(
        &mut self,
        part_id: PartId,
        gear: PartId,
        hook: PartTypeId,
        time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        AttachmentStore::attachment_find_part_attached_already(
            &mut self.mem,
            part_id,
            gear,
            hook,
            time,
        )
        .await
    }
}

#[async_trait::async_trait]
impl UsageStore for TestStravaStore {
    async fn get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> {
        UsageStore::get(&mut self.mem, uid).await
    }
    async fn delete(&mut self, usage: UsageId) -> TbResult<Usage> {
        UsageStore::delete(&mut self.mem, usage).await
    }
    async fn usages_delete(&mut self, usages: &[Usage]) -> TbResult<usize> {
        UsageStore::usages_delete(&mut self.mem, usages).await
    }
    async fn delete_all(&mut self) -> TbResult<usize> {
        UsageStore::delete_all(&mut self.mem).await
    }
    async fn update<U>(&mut self, usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        UsageStore::update(&mut self.mem, usage).await
    }
}

#[async_trait::async_trait]
impl ServiceStore for TestStravaStore {
    async fn create(&mut self, service: Service) -> TbResult<Service> {
        ServiceStore::create(&mut self.mem, service).await
    }
    async fn get(&mut self, service: ServiceId) -> TbResult<Service> {
        ServiceStore::get(&mut self.mem, service).await
    }
    async fn update(&mut self, service: Service) -> TbResult<Service> {
        ServiceStore::update(&mut self.mem, service).await
    }
    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> {
        ServiceStore::delete(&mut self.mem, service).await
    }
    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> {
        ServiceStore::services_delete(&mut self.mem, services).await
    }
    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> {
        ServiceStore::services_by_part(&mut self.mem, part).await
    }
}

#[async_trait::async_trait]
impl ServicePlanStore for TestStravaStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        ServicePlanStore::create(&mut self.mem, plan).await
    }
    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        ServicePlanStore::get(&mut self.mem, plan).await
    }
    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        ServicePlanStore::plan_update(&mut self.mem, plan).await
    }
    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        ServicePlanStore::delete(&mut self.mem, plan).await
    }
    async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize> {
        ServicePlanStore::serviceplans_delete(&mut self.mem, serviceplans).await
    }
    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> {
        ServicePlanStore::by_part(&mut self.mem, part).await
    }
    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> {
        ServicePlanStore::by_user(&mut self.mem, uid).await
    }
}
