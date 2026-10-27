//! In-memory test doubles for unit tests of the strava crate.
//!
//! `TestStravaStore` wraps the domain `MemStore` and adds in-memory
//! `strava_users` and `events` collections mirroring the SQL semantics.
//! `strava_session`/`strava_admin_session` build a domain `MockSession` that
//! seeds the `TokenCache` with a warm token; the free functions replay queued
//! JSON responses for exact request URIs.

#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use oauth2::RefreshToken;
use tb_domain::{
    MockSession,
    test_support::{MemStore, mock_session, mock_session_admin},
};
use time::OffsetDateTime;

use crate::event::Event;
use crate::*;

/// In-memory [`StravaDataStore`] delegating domain data to the domain [`MemStore`].
pub struct TestStravaStore {
    pub mem: MemStore,
    pub strava_users: HashMap<UserId, StravaUser>,
    pub events: Vec<Event>,
    next_event_id: i32,
}

impl Default for TestStravaStore {
    fn default() -> Self {
        Self::new()
    }
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

    /// Commit the wrapped domain transaction. Consumes the store, like the
    /// domain `MemStore::commit` and the production `SqlxConn::commit`.
    pub async fn commit(self) -> TbResult<()> {
        self.mem.commit().await
    }
}

#[async_trait::async_trait]
impl StravaDataStore for TestStravaStore {
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

    async fn strava_event_create(&mut self, mut e: Event) -> TbResult<()> {
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

    async fn stravauser_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
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

    async fn stravauser_create(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        let res = user.clone();
        self.strava_users.insert(user.tendabike_id, user);
        Ok(res)
    }

    async fn stravauser_update_token(
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

impl Store for TestStravaStore {}

impl StravaStore for TestStravaStore {}

// ─── Thread-local canned responses (feature-independent test seam) ──────────
//
// The thin free functions (`request_json`/`deauthorize`) take `&mut dyn
// Session` and cannot downcast to a scripted session, so the per-test
// canned responses live in a thread-local map the free functions consult.
// Each test runs on its own thread, so the map gives per-test isolation.
// `strava_session` (and the `tb_exec` gear tests) write to it; the free
// functions read from it. The process-wide `CANNED_STRAVA` global below is a
// separate seam for the seam suite's session, which runs in a spawned
// task on a thread this map never reaches.

/// The per-thread canned-response state (see the section docs).
#[derive(Default)]
struct CannedState {
    test_mode: bool,
    responses: HashMap<String, VecDeque<TbResult<String>>>,
    requests: Vec<String>,
    deauthorizes: Vec<StravaId>,
}

thread_local! {
    static CANNED: RefCell<CannedState> = RefCell::new(CannedState::default());
}

/// Begin a test's thread-local scope: marks the thread in test mode and clears
/// any canned responses/records a previous test left on this thread (per-test
/// isolation — each test runs on its own thread).
///
/// The process-wide `TokenCache` is *not* cleared here: the in-memory tests
/// run in parallel threads and all seed the same warm token (via
/// [`seed_token_cache`]), so a per-test clear would let one test's clear race
/// another test's `get_token` (a cache miss would force a real refresh).
pub fn begin_test() {
    CANNED.with(|c| {
        *c.borrow_mut() = CannedState::default();
        c.borrow_mut().test_mode = true;
    });
}

/// Seed the `TokenCache` with a warm token for `user_id` (test-only): prevents
/// the `get_token` free function from forcing a (network) refresh on the first
/// request (spec #602: the token source moved to `tb_strava`).
pub fn seed_token_cache(user_id: UserId, strava_id: StravaId, access_token: &str) {
    crate::session::TOKEN_CACHE
        .lock()
        .expect("the token cache is not poisoned")
        .insert(
            user_id,
            crate::session::TokenState::warm(strava_id, access_token),
        );
}

/// Whether the current thread is in test mode (`strava_session` was built,
/// or `begin_test` was called).
pub fn in_test_mode() -> bool {
    CANNED.with(|c| c.borrow().test_mode)
}

/// Queue a canned JSON response for an exact request URI (FIFO per URI).
pub fn queue(uri: &str, json: &str) {
    CANNED.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Ok(json.to_string()));
    });
}

/// Queue an error for an exact request URI.
pub fn queue_error(uri: &str, err: Error) {
    CANNED.with(|c| {
        c.borrow_mut()
            .responses
            .entry(uri.to_string())
            .or_default()
            .push_back(Err(err));
    });
}

/// Pop the canned response for `uri` (the free function's thread-local offline
/// seam).
///
/// Returns `None` when not in test mode (the caller falls through to the
/// process-wide `CANNED_STRAVA` global, then the real API). In test mode it
/// always returns `Some`: the queued body (or error), recording the request;
/// an unqueued URI records the request and yields the "unexpected request"
/// error.
pub fn pop_test_canned(uri: &str) -> Option<TbResult<String>> {
    CANNED.with(|c| {
        let mut c = c.borrow_mut();
        if !c.test_mode {
            return None;
        }
        c.requests.push(uri.to_string());
        Some(
            c.responses
                .get_mut(uri)
                .and_then(|q| q.pop_front())
                .unwrap_or(Err(Error::BadRequest(format!(
                    "unexpected strava request: {uri}"
                )))),
        )
    })
}

/// Record a deauthorization in the thread-local map (the free function's
/// offline seam).
pub fn record_deauthorize(strava: StravaId) {
    CANNED.with(|c| c.borrow_mut().deauthorizes.push(strava));
}

/// The requested URIs, in order (assert on these to verify API calls).
pub fn requests() -> Vec<String> {
    CANNED.with(|c| c.borrow().requests.clone())
}

/// The recorded `deauthorize()` calls.
pub fn deauthorizes() -> Vec<StravaId> {
    CANNED.with(|c| c.borrow().deauthorizes.clone())
}

/// A [`MockSession`] for `user` with a warm token seeded for `strava`
/// (test-only, no refresh): the thin free functions' `request_json`/
/// `deauthorize` replay the thread-local canned responses and record the
/// requested URIs / deauthorizations the tests assert on (the `queue`/
/// `requests`/`deauthorizes` free functions below).
pub fn strava_session(user: UserId, strava: StravaId) -> MockSession {
    begin_test();
    seed_token_cache(user, strava, "test-access-token");
    mock_session(user)
}

/// An admin [`MockSession`] for `user` with a warm token seeded for `strava`
/// (the admin-verification tests): like [`strava_session`], with `is_admin()`
/// returning `true`.
pub fn strava_admin_session(user: UserId, strava: StravaId) -> MockSession {
    begin_test();
    seed_token_cache(user, strava, "test-access-token");
    mock_session_admin(user)
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
impl PartNoteStore for TestStravaStore {
    async fn partnote_create_text(
        &mut self,
        part: PartId,
        name: String,
        created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        PartNoteStore::partnote_create_text(&mut self.mem, part, name, created).await
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
        PartNoteStore::partnote_create_file(
            &mut self.mem,
            part,
            name,
            mime,
            filename,
            size,
            data,
            created,
        )
        .await
    }
    async fn partnote_all_by_part(&mut self, part: PartId) -> TbResult<Vec<PartNote>> {
        PartNoteStore::partnote_all_by_part(&mut self.mem, part).await
    }
    async fn partnote_get(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        PartNoteStore::partnote_get(&mut self.mem, id).await
    }
    async fn partnote_file(&mut self, id: PartNoteId) -> TbResult<Vec<u8>> {
        PartNoteStore::partnote_file(&mut self.mem, id).await
    }
    async fn partnote_update_text(&mut self, id: PartNoteId, name: String) -> TbResult<PartNote> {
        PartNoteStore::partnote_update_text(&mut self.mem, id, name).await
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
        PartNoteStore::partnote_update_file(&mut self.mem, id, name, mime, filename, size, data)
            .await
    }
    async fn partnote_remove_file(&mut self, id: PartNoteId) -> TbResult<PartNote> {
        PartNoteStore::partnote_remove_file(&mut self.mem, id).await
    }
    async fn partnote_delete(&mut self, id: PartNoteId) -> TbResult<PartNoteId> {
        PartNoteStore::partnote_delete(&mut self.mem, id).await
    }
}

#[async_trait::async_trait]
impl UserStore for TestStravaStore {
    async fn user_get(&mut self, uid: UserId) -> TbResult<User> {
        UserStore::user_get(&mut self.mem, uid).await
    }
    async fn user_create(
        &mut self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        UserStore::user_create(&mut self.mem, firstname, lastname, avatar).await
    }
    async fn user_update(
        &mut self,
        uid: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
    ) -> TbResult<User> {
        UserStore::user_update(&mut self.mem, uid, firstname, lastname, avatar).await
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
    async fn service_create(&mut self, service: Service) -> TbResult<Service> {
        ServiceStore::service_create(&mut self.mem, service).await
    }
    async fn service_get(&mut self, service: ServiceId) -> TbResult<Service> {
        ServiceStore::service_get(&mut self.mem, service).await
    }
    async fn service_update(&mut self, service: Service) -> TbResult<Service> {
        ServiceStore::service_update(&mut self.mem, service).await
    }
    async fn service_delete(&mut self, service: ServiceId) -> TbResult<usize> {
        ServiceStore::service_delete(&mut self.mem, service).await
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
    async fn serviceplan_create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        ServicePlanStore::serviceplan_create(&mut self.mem, plan).await
    }
    async fn serviceplan_get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> {
        ServicePlanStore::serviceplan_get(&mut self.mem, plan).await
    }
    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> {
        ServicePlanStore::plan_update(&mut self.mem, plan).await
    }
    async fn serviceplan_delete(&mut self, plan: ServicePlanId) -> TbResult<usize> {
        ServicePlanStore::serviceplan_delete(&mut self.mem, plan).await
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

// ─── Canned Strava responses (feature `test-support`) ──────────────────────
//
// The seam suite runs the real loop, whose session is a plain `Session`, so
// the suite maps canned responses for the URIs the loop asks for — an empty
// activity list makes the sync's activities fetch succeed offline (no new
// activities: the loop's processing consumes the event and commits).
// `request_json` consults [`global_canned_response`] before reaching the
// Strava API.

/// The canned Strava responses, keyed by URI (see the section docs).
pub static CANNED_STRAVA: std::sync::LazyLock<std::sync::Mutex<HashMap<String, String>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Maps a canned response for `uri` (test-only, see [`CANNED_STRAVA`]).
pub fn canned_strava(uri: &str, json: &str) {
    CANNED_STRAVA
        .lock()
        .expect("the canned Strava responses are not poisoned")
        .insert(uri.to_string(), json.to_string());
}

/// The process-wide canned response for `uri`, if one is mapped (test-only,
/// the seam-suite's global seam — distinct from the thread-local
/// [`pop_test_canned`]).
pub fn global_canned_response(uri: &str) -> Option<String> {
    CANNED_STRAVA
        .lock()
        .expect("the canned Strava responses are not poisoned")
        .get(uri)
        .cloned()
}
