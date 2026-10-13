//! The in-crate minimal test domain for the strava crate's unit tests
//! (issue #530).
//!
//! [`TestDomain`] is a stub [`Domain`]: the real `tb_domain` entity types
//! (the tests name and assert on their fields) with canned operations. The
//! stub ops never call the store's trait surface — they write the domain
//! half of the shared state directly — which is why [`TestStravaStore`]
//! carries no domain sub-trait methods: just the 15 [`StravaStore`] methods
//! (the event queue + user map the strava tests drive) and the 2 no-op
//! [`Txn`] commit/rollback.
//!
//! [`TestStravaSession`] is a scripted [`StravaSession`] that replays
//! queued JSON responses for exact request URIs.
//!
//! Semantics carry forward from #510: no transaction buffer (every write
//! lands directly in the shared state), `Arc<Mutex<…>>` shared state, no
//! fault injection.

#![allow(clippy::too_many_arguments)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use oauth2::RefreshToken;
use serde::de::DeserializeOwned;
use tb_exec::Txn;
use time::OffsetDateTime;

use crate::event::Event;
use crate::*;

/// The shared state of the tiny store: the strava half (the event queue +
/// user map the strava tests drive) and the domain half the stub ops of
/// [`TestDomain`] write (users, parts, activities).
#[derive(Debug, Default)]
pub struct State {
    pub strava_users: HashMap<UserId, StravaUser>,
    pub events: Vec<Event>,
    pub users: HashMap<UserId, User>,
    pub parts: HashMap<PartId, Part>,
    pub activities: HashMap<ActivityId, Activity>,
    next_event_id: i32,
    next_user_id: i32,
    next_part_id: i32,
}

/// The tiny store of the strava test suite (issue #530): the 15
/// [`StravaStore`] methods with their real queue/user-map behavior, plus
/// the 2 no-op [`Txn`] commit/rollback — ≈17 methods, no domain sub-trait
/// methods.
pub struct TestStravaStore {
    pub state: Arc<Mutex<State>>,
}

impl TestStravaStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
        }
    }

    pub fn insert_user(&mut self, user: StravaUser) {
        self.state
            .lock()
            .unwrap()
            .strava_users
            .insert(user.tendabike_id, user);
    }

    pub fn event_count(&self) -> usize {
        self.state.lock().unwrap().events.len()
    }

    /// A snapshot of the event queue (cloned; the state is mutex-guarded).
    pub fn events(&self) -> Vec<Event> {
        self.state.lock().unwrap().events.clone()
    }

    /// A snapshot of the strava users (cloned).
    pub fn strava_users(&self) -> Vec<StravaUser> {
        self.state
            .lock()
            .unwrap()
            .strava_users
            .values()
            .cloned()
            .collect()
    }

    /// The domain half the stub ops wrote: the user, part, or activity with
    /// the id (cloned).
    pub fn user(&self, id: &UserId) -> Option<User> {
        self.state.lock().unwrap().users.get(id).cloned()
    }

    pub fn part(&self, id: &PartId) -> Option<Part> {
        self.state.lock().unwrap().parts.get(id).cloned()
    }

    pub fn activities(&self) -> Vec<Activity> {
        self.state
            .lock()
            .unwrap()
            .activities
            .values()
            .cloned()
            .collect()
    }
}

#[async_trait::async_trait]
impl StravaStore<TestDomain> for TestStravaStore {
    async fn stravaid_get_user_id(&mut self, who: i32) -> Result<i32, Error> {
        self.state
            .lock()
            .unwrap()
            .strava_users
            .values()
            .find(|u| u.tendabike_id == UserId::from(who))
            .map(|u| i32::from(u.id))
            .ok_or_else(|| Error::NotFound(format!("strava user {who} not found")))
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> Result<(), Error> {
        self.state
            .lock()
            .unwrap()
            .events
            .retain(|e| e.id != event_id);
        Ok(())
    }

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> Result<(), Error> {
        for e in &mut self.state.lock().unwrap().events {
            if e.id == e_id {
                e.event_time = e_time;
            }
        }
        Ok(())
    }

    async fn stravaevent_store(&mut self, mut e: Event) -> Result<(), Error> {
        let mut state = self.state.lock().unwrap();
        state.next_event_id += 1;
        e.id = Some(state.next_event_id);
        state.events.push(e);
        Ok(())
    }

    async fn strava_event_get_next_for_user(
        &mut self,
        user: StravaId,
    ) -> Result<Option<Event>, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.owner_id == user || e.owner_id == StravaId::default())
            .min_by_key(|e| e.event_time)
            .cloned())
    }

    async fn strava_event_get_later(
        &mut self,
        obj_id: i64,
        oid: StravaId,
    ) -> Result<Vec<Event>, Error> {
        let mut res: Vec<Event> = self
            .state
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.object_id == obj_id && e.owner_id == oid)
            .cloned()
            .collect();
        res.sort_by_key(|e| e.event_time);
        Ok(res)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> Result<(), Error> {
        self.state
            .lock()
            .unwrap()
            .events
            .retain(|e| !values.contains(&e.id));
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> Result<Vec<StravaUser>, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .strava_users
            .values()
            .cloned()
            .collect())
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> Result<StravaUser, Error> {
        self.state
            .lock()
            .unwrap()
            .strava_users
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("user {id} not registered with strava")))
    }

    async fn stravauser_get_by_stravaid(
        &mut self,
        id: &StravaId,
    ) -> Result<Option<StravaUser>, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .strava_users
            .values()
            .find(|u| u.id == *id)
            .cloned())
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> Result<StravaUser, Error> {
        let res = user.clone();
        self.state
            .lock()
            .unwrap()
            .strava_users
            .insert(user.tendabike_id, user);
        Ok(res)
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> Result<StravaUser, Error> {
        let mut state = self.state.lock().unwrap();
        let user = state
            .strava_users
            .values_mut()
            .find(|u| u.id == stravaid)
            .ok_or_else(|| Error::NotFound(format!("strava user {stravaid} not found")))?;
        user.refresh_token = refresh.map(|r| RefreshToken::new(r.clone()));
        Ok(user.clone())
    }

    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> Result<i64, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|e| &e.owner_id == user)
            .count() as i64)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> Result<usize, Error> {
        let mut state = self.state.lock().unwrap();
        let before = state.events.len();
        state.events.retain(|e| &e.owner_id != user);
        Ok(before - state.events.len())
    }

    async fn stravauser_delete(&mut self, user: UserId) -> Result<usize, Error> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .strava_users
            .remove(&user)
            .is_some() as usize)
    }
}

#[async_trait::async_trait]
impl Txn<TestDomain> for TestStravaStore {
    async fn commit(self) -> Result<(), Error> {
        Ok(())
    }

    async fn rollback(self) -> Result<(), Error> {
        Ok(())
    }
}

/// The minimal test domain (issue #530): the real `tb_domain` entity types
/// with canned operations. The ops never call the store's trait surface —
/// they write the domain half of [`TestStravaStore`]'s shared state
/// directly — so the store's domain half stays tiny.
pub struct TestDomain;

#[async_trait::async_trait]
impl Domain for TestDomain {
    type Error = Error;
    type Summary = Summary;
    type Store = TestStravaStore;
    type ApiWrite = ApiWrite;
    type WriteOutcome = WriteOutcome;
    type Activity = Activity;
    type ActivityId = ActivityId;
    type ActTypeId = ActTypeId;
    type PartId = PartId;
    type PartTypeId = PartTypeId;
    type UserId = UserId;
    type Part = Part;
    type Usage = Usage;

    async fn upsert_activity(
        activity: Activity,
        _user: &impl Session,
        store: &mut TestStravaStore,
    ) -> Result<Summary, Error> {
        store
            .state
            .lock()
            .unwrap()
            .activities
            .insert(activity.id, activity.clone());
        let mut summary = Summary::default();
        summary.activities.upsert(activity);
        Ok(summary)
    }

    async fn read_activity(
        id: ActivityId,
        _session: &impl Session,
        store: &mut TestStravaStore,
    ) -> Result<Activity, Error> {
        store
            .state
            .lock()
            .unwrap()
            .activities
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("activity {id} not found")))
    }

    async fn delete_activity(
        id: ActivityId,
        session: &impl Session,
        store: &mut TestStravaStore,
    ) -> Result<Summary, Error> {
        let mut state = store.state.lock().unwrap();
        let act = state
            .activities
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("activity {id} not found")))?;
        if act.user_id != session.user_id() {
            return Err(Error::Forbidden(format!(
                "user {} cannot delete activity {id}",
                session.user_id()
            )));
        }
        state.activities.remove(&id);
        let mut summary = Summary::default();
        summary.activities.tombstone(id);
        Ok(summary)
    }

    async fn create_part(
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: &impl Session,
        store: &mut TestStravaStore,
    ) -> Result<Part, Error> {
        let mut state = store.state.lock().unwrap();
        state.next_part_id += 1;
        let id = PartId::from(state.next_part_id);
        let part = Part {
            id,
            owner: user.user_id(),
            what,
            name,
            vendor,
            model,
            purchase,
            last_used: OffsetDateTime::now_utc(),
            disposed_at: None,
            usage: UsageId::default(),
            source,
            shop: None,
        };
        state.parts.insert(id, part.clone());
        Ok(part)
    }

    async fn part_by_source(
        source: &str,
        store: &mut TestStravaStore,
    ) -> Result<Option<Part>, Error> {
        Ok(store
            .state
            .lock()
            .unwrap()
            .parts
            .values()
            .find(|p| p.source.as_deref() == Some(source))
            .cloned())
    }

    async fn exec(
        _write: ApiWrite,
        _session: &mut impl Session,
        _store: &mut TestStravaStore,
    ) -> Result<WriteOutcome, Error> {
        Ok(WriteOutcome::Summary(Summary::default()))
    }

    fn summary_is_empty(summary: &Summary) -> bool {
        summary.is_empty()
    }

    fn write_outcome_summary(outcome: &WriteOutcome) -> &Summary {
        outcome.summary()
    }

    fn any_failure(err: anyhow::Error) -> Error {
        Error::AnyFailure(err)
    }

    fn strava_failure(err: &Error) -> StravaFailure {
        match err {
            Error::TryAgain(msg) => StravaFailure::TryAgain(msg.to_string()),
            Error::NotAuth(msg) => StravaFailure::NotAuth(msg.clone()),
            Error::DatabaseFailure(_) => StravaFailure::DatabaseFailure,
            Error::NotFound(_) => StravaFailure::NotFound,
            _ => StravaFailure::Other,
        }
    }

    fn bad_request(msg: String) -> Error {
        Error::BadRequest(msg)
    }

    fn not_found(msg: String) -> Error {
        Error::NotFound(msg)
    }

    fn part_id(part: &Part) -> PartId {
        part.id
    }

    fn activity_id(activity: &Activity) -> ActivityId {
        activity.id
    }

    async fn read_part(
        id: PartId,
        session: &impl Session,
        store: &mut TestStravaStore,
    ) -> Result<Part, Error> {
        let part = store
            .state
            .lock()
            .unwrap()
            .parts
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("part {id} not found")))?;
        let user = session.user_id();
        if part.owner != user && session.shop().is_none() {
            return Err(Error::Forbidden(format!(
                "user {user} cannot access part {id}"
            )));
        }
        Ok(part)
    }

    async fn create_user(
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut TestStravaStore,
    ) -> Result<UserId, Error> {
        let mut state = store.state.lock().unwrap();
        state.next_user_id += 1;
        let id = UserId::from(state.next_user_id);
        state.users.insert(
            id,
            User {
                id,
                name: lastname,
                firstname,
                avatar: avatar.clone(),
                is_admin: false,
                onboarding_status: OnboardingStatus::default(),
            },
        );
        Ok(id)
    }

    async fn update_user(
        id: UserId,
        firstname: String,
        lastname: String,
        avatar: &Option<String>,
        store: &mut TestStravaStore,
    ) -> Result<UserId, Error> {
        let mut state = store.state.lock().unwrap();
        let user = state
            .users
            .get_mut(&id)
            .ok_or_else(|| Error::NotFound(format!("user {id} not found")))?;
        user.firstname = firstname;
        user.name = lastname;
        user.avatar = avatar.clone();
        Ok(id)
    }

    async fn delete_user(id: UserId, store: &mut TestStravaStore) -> Result<(), Error> {
        // The concrete op cascades without an existence check: the user's
        // parts and activities go with the row, a missing row is a no-op.
        let mut state = store.state.lock().unwrap();
        state.users.remove(&id);
        state.parts.retain(|_, p| p.owner != id);
        state.activities.retain(|_, a| a.user_id != id);
        Ok(())
    }

    async fn read_stats(id: UserId, store: &mut TestStravaStore) -> Result<Stat, Error> {
        let user = store
            .state
            .lock()
            .unwrap()
            .users
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("user {id} not found")))?;
        Ok(Stat::new(user))
    }

    fn make_activity(
        id: ActivityId,
        user_id: UserId,
        what: ActTypeId,
        name: String,
        start: OffsetDateTime,
        duration: i32,
        time: Option<i32>,
        distance: Option<i32>,
        climb: Option<i32>,
        descend: Option<i32>,
        energy: Option<i32>,
        gear: Option<PartId>,
        device_name: Option<String>,
        external_id: Option<String>,
    ) -> Activity {
        Activity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
            device_name,
            external_id,
        }
    }

    fn make_activity_id(id: i64) -> ActivityId {
        ActivityId::new(id)
    }

    fn make_part_id(id: i32) -> PartId {
        PartId::from(id)
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

    fn set_shop(&mut self, _shop: Option<ShopId>) -> Result<(), Error> {
        Ok(())
    }

    fn is_admin(&self) -> bool {
        self.admin
    }
}

#[async_trait::async_trait]
impl StravaSession<TestDomain> for TestStravaSession {
    fn strava_id(&self) -> StravaId {
        self.strava
    }

    fn tb_id(&self) -> UserId {
        self.user
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        _store: &mut impl StravaStore<TestDomain>,
    ) -> Result<T, Error> {
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

    async fn deauthorize(
        &mut self,
        _store: &mut impl StravaStore<TestDomain>,
    ) -> Result<(), Error> {
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
