//! Test support for the Strava crate.
//!
//! The store seam is a `mockall` `mock!` witness ([`MockStravaConn`],
//! `#[cfg(test)]`): a single type accumulating the full `Store` (nine
//! sub-traits + the marker) plus the `StravaStore` surface — no `Txn`, since
//! the Strava-logic free functions take a bare `&mut impl StravaStore`. The
//! Strava-logic tests (into_partid, activity sync, gear hydration, event
//! processing, deauthorize) run the logic for real and assert on canned
//! `mock!` outcomes; the old in-memory delegation is gone.
//!
//! The HTTP seam (the thread-local canned responses, the `TokenCache` seeder,
//! and the `CANNED_STRAVA` global) is feature-independent: the thin free
//! functions (`request_json`/`deauthorize`) consult it in test mode, and the
//! `test-support` feature build (the `tb_exec`/`tb_sqlx` seam suites) maps
//! canned responses for the URIs the real loop asks for.

#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

#[cfg(test)]
use crate::event::Event;
#[cfg(test)]
use mockall::mock;
use oauth2::RefreshToken;
use tb_domain::{
    MockSession,
    test_support::{mock_session, mock_session_admin},
};
#[cfg(test)]
use time::OffsetDateTime;

use crate::*;

// --- The `mock!` store witness (issue #632) ---
//
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
//
// Test-support only: `mockall` is a dev-dependency of `tb_strava` only, so no
// production `pub` surface is widened.
#[cfg(test)]
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

// --- Canned-value builders for the `mock!` outcomes ---
//
// The Strava-logic tests return canned domain values from the witness's
// `expect_*` hooks; these builders give those values the fields the logic
// reads (ids, owner, type, source) and fill the rest with inert defaults.

/// A canned [`Part`] with the given id, owner, type, and Strava source.
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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
