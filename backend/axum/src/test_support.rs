//! Test infrastructure for the `tb_axum` crate.
//!
//! Builds the production router stack without a database: the state carries a
//! `FakeSource` whose `begin` fails fast with a `DatabaseFailure`, so any
//! handler that touches the database returns a 500, and sessions are backed by
//! a `MemoryStore` that tests can seed.

#![allow(clippy::too_many_arguments)]

use std::sync::{Arc, Once};

use axum::Router;
use http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tb_domain::*;
use tb_exec::{Txn, TxnSource};
use tb_strava::{StravaId, StravaStore, StravaUser, event::Event};
use time::OffsetDateTime;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use crate::appstate::AppState;
use crate::routes;
use crate::strava::RequestSession;

const SESSION_KEY: &str = "session";

/// Dummies the OAuth client environment variables so the `STRAVACLIENT` lazy static
/// does not panic when it is first used in a test.
fn set_oauth_env_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        std::env::set_var("CLIENT_ID", "test-client-id");
        std::env::set_var("CLIENT_SECRET", "test-client-secret");
    });
}

/// Builds the production routes with a fast-failing source (every `begin` is a
/// `DatabaseFailure`, so DB-reaching routes answer 500) and a `MemoryStore`
/// session layer.
pub(crate) fn test_app(store: MemoryStore) -> Router {
    set_oauth_env_once();
    let session_layer = SessionManagerLayer::new(store);
    routes(AppState::new(FakeSource)).layer(session_layer)
}

/// Creates a session in `store` containing `value` and returns the cookie header value
/// that authenticates with it.
pub(crate) async fn cookie_for(store: &MemoryStore, value: RequestSession) -> String {
    let session = Session::new(None, Arc::new(store.clone()), None);
    session
        .insert(SESSION_KEY, &value)
        .await
        .expect("session insert");
    session.save().await.expect("session save");
    let id = session.id().expect("session id");
    format!("id={id}")
}

/// Cookie header value for a regular (non-admin) user.
pub(crate) async fn user_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(false)).await
}

/// Cookie header value for an admin user.
pub(crate) async fn admin_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(true)).await
}

/// Runs a single request against `app` and returns its status, headers and body.
pub(crate) async fn run(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    run_with_body(app, method, uri, cookie, None).await
}

/// Runs a single request with a JSON body against `app`.
pub(crate) async fn run_json(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    run_with_body(app, method, uri, cookie, Some(body)).await
}

async fn run_with_body(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    json: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "localhost");
    let body = if let Some(json) = json {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        axum::body::Body::from(json.as_bytes().to_vec())
    } else {
        axum::body::Body::empty()
    };
    let mut req = builder.body(body).expect("valid request");
    if let Some(cookie) = cookie {
        req.headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("valid cookie"));
    }
    let res = app
        .oneshot(req)
        .await
        .expect("oneshot request should succeed");
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.into_body().collect().await.expect("body").to_bytes();
    (status, headers, body.to_vec())
}

/// A fake connection: the nine `Store` sub-traits, `StravaStore` (all methods
/// `unimplemented!()` — they are never reached because `begin` fails first),
/// and `Txn` with `commit`/`rollback` returning a `DatabaseFailure`.
struct FakeConn;

impl Store for FakeConn {}

#[async_trait::async_trait]
impl PartStore for FakeConn {
    async fn partid_get_part(&mut self, _pid: PartId) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_get_all_for_userid(&mut self, _uid: &UserId) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn part_create(
        &mut self,
        _what: PartTypeId,
        _name: String,
        _vendor: String,
        _model: String,
        _purchase: OffsetDateTime,
        _source: Option<String>,
        _usage: UsageId,
        _owner: UserId,
        _shop: Option<ShopId>,
    ) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_update(&mut self, _part: Part) -> TbResult<Part> {
        unimplemented!()
    }
    async fn part_delete(&mut self, _part: PartId) -> TbResult<PartId> {
        unimplemented!()
    }
    async fn parts_delete(&mut self, _parts: &[Part]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn partid_get_by_source(&mut self, _strava_id: &str) -> TbResult<Option<PartId>> {
        unimplemented!()
    }
    async fn parts_register_shop(
        &mut self,
        _shop_id: ShopId,
        _part_id: Vec<PartId>,
    ) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn parts_unregister_shop(&mut self, _part_ids: Vec<PartId>) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
    async fn shop_get_parts(&mut self, _shop_id: ShopId) -> TbResult<Vec<Part>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl UserStore for FakeConn {
    async fn get(&mut self, _uid: UserId) -> TbResult<User> {
        unimplemented!()
    }
    async fn create(
        &mut self,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        unimplemented!()
    }
    async fn update(
        &mut self,
        _uid: &UserId,
        _firstname: &str,
        _lastname: &str,
        _avatar: &Option<String>,
    ) -> TbResult<User> {
        unimplemented!()
    }
    async fn user_delete(&mut self, _user: &UserId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn update_onboarding_status(
        &mut self,
        _uid: &UserId,
        _status: OnboardingStatus,
    ) -> TbResult<User> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ShopStore for FakeConn {
    async fn shop_create(
        &mut self,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
        _owner: UserId,
    ) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_get(&mut self, _id: ShopId) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_update(
        &mut self,
        _id: ShopId,
        _name: String,
        _description: Option<String>,
        _auto_approve: bool,
    ) -> TbResult<Shop> {
        unimplemented!()
    }
    async fn shop_delete(&mut self, _id: ShopId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn shops_get_all_for_user(&mut self, _user_id: UserId) -> TbResult<Vec<Shop>> {
        unimplemented!()
    }
    async fn shops_search(&mut self, _query: &str) -> TbResult<Vec<Shop>> {
        unimplemented!()
    }
    async fn subscription_create(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
        _message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_get(&mut self, _id: SubscriptionId) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_find_active(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscription_find_pending(
        &mut self,
        _shop_id: ShopId,
        _user_id: UserId,
    ) -> TbResult<Option<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscription_update_status(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_approve(
        &mut self,
        _id: SubscriptionId,
        _status: SubscriptionStatus,
        _response_message: Option<String>,
    ) -> TbResult<ShopSubscription> {
        unimplemented!()
    }
    async fn subscription_delete(&mut self, _id: SubscriptionId) -> TbResult<()> {
        unimplemented!()
    }
    async fn subscriptions_for_shop(
        &mut self,
        _shop_id: ShopId,
    ) -> TbResult<Vec<ShopSubscription>> {
        unimplemented!()
    }
    async fn subscriptions_for_user(
        &mut self,
        _user_id: UserId,
    ) -> TbResult<Vec<ShopSubscription>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ActivityStore for FakeConn {
    async fn activity_create(&mut self, _act: Activity) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_read_by_id(&mut self, _aid: ActivityId) -> TbResult<Option<Activity>> {
        unimplemented!()
    }
    async fn activity_update(&mut self, _act: Activity) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_delete(&mut self, _aid: ActivityId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn activities_delete(&mut self, _activities: &[Activity]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn get_all(&mut self, _uid: &UserId) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn activities_find_by_gear_and_time(
        &mut self,
        _part: PartId,
        _begin: OffsetDateTime,
        _end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn get_by_user_and_time(
        &mut self,
        _uid: UserId,
        _rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        unimplemented!()
    }
    async fn activity_set_gear_if_null(
        &mut self,
        _user: UserId,
        _types: Vec<ActTypeId>,
        _partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl AttachmentStore for FakeConn {
    async fn attachment_create(&mut self, _att: Attachment) -> TbResult<Attachment> {
        unimplemented!()
    }
    async fn delete(&mut self, _att: Attachment) -> TbResult<Attachment> {
        unimplemented!()
    }
    async fn attachments_delete_by_parts(&mut self, _parts: &[Part]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn attachment_get_by_gear_and_time(
        &mut self,
        _act_gear: PartId,
        _start: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachments_all_by_part(&mut self, _id: PartId) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachment_get_by_part_and_time(
        &mut self,
        _pid: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn assembly_get_by_types_time_and_gear(
        &mut self,
        _types: Vec<PartTypeId>,
        _gear: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Vec<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_part_of_type_at_hook_and_time(
        &mut self,
        _what: PartTypeId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_successor(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
        _what: PartTypeId,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_later_attachment_for_part(
        &mut self,
        _part_id: PartId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
    async fn attachment_find_part_attached_already(
        &mut self,
        _part_id: PartId,
        _gear: PartId,
        _hook: PartTypeId,
        _time: OffsetDateTime,
    ) -> TbResult<Option<Attachment>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl PartNoteStore for FakeConn {
    async fn partnote_create_text(
        &mut self,
        _part: PartId,
        _name: String,
        _created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_create_file(
        &mut self,
        _part: PartId,
        _name: String,
        _mime: String,
        _filename: Option<String>,
        _size: i64,
        _data: Vec<u8>,
        _created: OffsetDateTime,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_all_by_part(&mut self, _part: PartId) -> TbResult<Vec<PartNote>> {
        unimplemented!()
    }
    async fn partnote_get(&mut self, _id: PartNoteId) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_file(&mut self, _id: PartNoteId) -> TbResult<Vec<u8>> {
        unimplemented!()
    }
    async fn partnote_update_text(&mut self, _id: PartNoteId, _name: String) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_update_file(
        &mut self,
        _id: PartNoteId,
        _name: String,
        _mime: String,
        _filename: Option<String>,
        _size: i64,
        _data: Vec<u8>,
    ) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_remove_file(&mut self, _id: PartNoteId) -> TbResult<PartNote> {
        unimplemented!()
    }
    async fn partnote_delete(&mut self, _id: PartNoteId) -> TbResult<PartNoteId> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl UsageStore for FakeConn {
    async fn get(&mut self, _uid: UsageId) -> TbResult<Option<Usage>> {
        unimplemented!()
    }
    async fn update<U>(&mut self, _usage: &[U]) -> TbResult<usize>
    where
        U: std::borrow::Borrow<Usage> + Sync,
    {
        unimplemented!()
    }
    async fn delete(&mut self, _usage: UsageId) -> TbResult<Usage> {
        unimplemented!()
    }
    async fn usages_delete(&mut self, _usages: &[Usage]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn delete_all(&mut self) -> TbResult<usize> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ServiceStore for FakeConn {
    async fn create(&mut self, _service: Service) -> TbResult<Service> {
        unimplemented!()
    }
    async fn get(&mut self, _service: ServiceId) -> TbResult<Service> {
        unimplemented!()
    }
    async fn update(&mut self, _service: Service) -> TbResult<Service> {
        unimplemented!()
    }
    async fn delete(&mut self, _service: ServiceId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn services_delete(&mut self, _services: &[Service]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn services_by_part(&mut self, _part: PartId) -> TbResult<Vec<Service>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl ServicePlanStore for FakeConn {
    async fn create(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn get(&mut self, _plan: ServicePlanId) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn plan_update(&mut self, _plan: ServicePlan) -> TbResult<ServicePlan> {
        unimplemented!()
    }
    async fn delete(&mut self, _plan: ServicePlanId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn serviceplans_delete(&mut self, _serviceplans: &[ServicePlan]) -> TbResult<usize> {
        unimplemented!()
    }
    async fn by_part(&mut self, _part: PartId) -> TbResult<Vec<ServicePlan>> {
        unimplemented!()
    }
    async fn by_user(&mut self, _uid: UserId) -> TbResult<Vec<ServicePlan>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl StravaStore for FakeConn {
    async fn stravaid_get_user_id(&mut self, _who: i32) -> TbResult<i32> {
        unimplemented!()
    }
    async fn strava_event_delete(&mut self, _event_id: Option<i32>) -> TbResult<()> {
        unimplemented!()
    }
    async fn strava_event_set_time(&mut self, _e_id: Option<i32>, _e_time: i64) -> TbResult<()> {
        unimplemented!()
    }
    async fn stravaevent_store(&mut self, _e: Event) -> TbResult<()> {
        unimplemented!()
    }
    async fn strava_event_get_next_for_user(&mut self, _user: StravaId) -> TbResult<Option<Event>> {
        unimplemented!()
    }
    async fn strava_event_get_later(
        &mut self,
        _obj_id: i64,
        _oid: StravaId,
    ) -> TbResult<Vec<Event>> {
        unimplemented!()
    }
    async fn strava_events_delete_batch(&mut self, _values: Vec<Option<i32>>) -> TbResult<()> {
        unimplemented!()
    }
    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        unimplemented!()
    }
    async fn stravauser_get_by_tbid(&mut self, _id: UserId) -> TbResult<StravaUser> {
        unimplemented!()
    }
    async fn stravauser_get_by_stravaid(&mut self, _id: &StravaId) -> TbResult<Option<StravaUser>> {
        unimplemented!()
    }
    async fn stravauser_new(&mut self, _user: StravaUser) -> TbResult<StravaUser> {
        unimplemented!()
    }
    async fn stravaid_update_token(
        &mut self,
        _stravaid: StravaId,
        _refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        unimplemented!()
    }
    async fn strava_events_get_count_for_user(&mut self, _user: &StravaId) -> TbResult<i64> {
        unimplemented!()
    }
    async fn strava_events_delete_for_user(&mut self, _user: &StravaId) -> TbResult<usize> {
        unimplemented!()
    }
    async fn stravauser_delete(&mut self, _user: UserId) -> TbResult<usize> {
        unimplemented!()
    }
}
#[async_trait::async_trait]
impl Txn for FakeConn {
    async fn commit(self) -> TbResult<()> {
        Err(Error::DatabaseFailure(anyhow::anyhow!(
            "fake connection: no database"
        )))
    }
    async fn rollback(self) -> TbResult<()> {
        Err(Error::DatabaseFailure(anyhow::anyhow!(
            "fake connection: no database"
        )))
    }
}

/// A fake source: every `begin` fails with a `DatabaseFailure`, so any handler
/// that opens a transaction answers a 500 instead of reaching a database.
#[derive(Clone)]
struct FakeSource;

#[async_trait::async_trait]
impl TxnSource for FakeSource {
    type Conn = FakeConn;

    async fn begin(&self) -> TbResult<Self::Conn> {
        Err(Error::DatabaseFailure(anyhow::anyhow!(
            "fake source: no database"
        )))
    }
}
