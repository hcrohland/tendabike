use async_recursion::async_recursion;
use log::error;
use std::collections::HashMap;

use crate::activity::StravaActivity;
use crate::*;

#[derive(Debug, Serialize, Deserialize)]
/// A struct representing an incoming Strava event.
pub struct InEvent {
    // Always either "activity" or "athlete."
    object_type: String,
    object_id: i64,
    // Always "create," "update," or "delete."
    aspect_type: String,
    // For activity update strava_events,
    //     keys can contain "title," "type,"
    //     and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone).
    // For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    updates: HashMap<String, String>,
    // The athlete's ID.
    owner_id: i32,
    // The push subscription ID that is receiving this event.
    subscription_id: i32,
    // The time that the event occurred.
    event_time: i64,
}

impl InEvent {
    /// Converts an incoming Strava event into an `Event` struct.
    ///
    /// # Arguments
    ///
    /// * `self` - An instance of `InEvent`.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Event` struct if the conversion is successful, or an `anyhow::Error` if it fails.
    pub async fn into_event(self, store: &mut impl StravaStore) -> TbResult<Event> {
        if StravaId::read(&self.owner_id.into(), store)
            .await?
            .is_none()
        {
            return Err(Error::BadRequest(format!(
                "Unknown event owner received: {self:?}"
            )));
        }
        let InEvent {
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id,
            subscription_id,
            event_time,
        } = self;
        let object_type = object_type.try_into()?;
        let aspect_type = aspect_type.try_into()?;
        let owner_id = owner_id.into();
        Ok(Event {
            id: None,
            object_type,
            object_id,
            aspect_type,
            owner_id,
            subscription_id,
            event_time,
            updates,
        })
    }

    /// Accepts the event: validates the owner, then either processes an
    /// athlete (de)authorization or stores the event in the DB queue.
    ///
    /// Returns the owner's TendaBike [`UserId`] when the event was queued (so
    /// the caller can wake that user's executor), or `None` for an athlete
    /// (de)authorization (nothing was queued, so there is nothing to wake).
    pub async fn accept(self, store: &mut impl StravaStore) -> TbResult<Option<UserId>> {
        let user_id = StravaId::read(&self.owner_id.into(), store)
            .await?
            .map(|user| user.tendabike_id)
            .ok_or_else(|| Error::BadRequest(format!("Unknown event owner received: {self:?}")))?;
        let event = self.into_event(store).await?;
        if event.object_type == ObjectType::Athlete {
            event.process_user(store).await?;
            Ok(None)
        } else {
            store.stravaevent_store(event).await?;
            Ok(Some(user_id))
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjectType {
    #[default]
    Activity,
    Athlete,
    Sync,
    Stop,
}

impl TryFrom<String> for ObjectType {
    type Error = tb_domain::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        use ObjectType::*;
        Ok(match value.as_str() {
            "activity" => Activity,
            "athlete" => Athlete,
            "stop" => Stop,
            "sync" => Sync,
            _ => return Err(Error::BadRequest(format!("Unknown object type {value}"))),
        })
    }
}

impl From<ObjectType> for String {
    fn from(value: ObjectType) -> Self {
        use ObjectType::*;
        String::from(match value {
            Activity => "activity",
            Athlete => "athlete",
            Stop => "stop",
            Sync => "sync",
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub enum AspectType {
    #[default]
    Create,
    Update,
    Delete,
}

impl TryFrom<String> for AspectType {
    type Error = tb_domain::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "create" => AspectType::Create,
            "update" => AspectType::Update,
            "delete" => AspectType::Delete,
            _ => return Err(Error::BadRequest(format!("Unknown aspect type {value}"))),
        })
    }
}

impl From<AspectType> for String {
    fn from(value: AspectType) -> Self {
        use AspectType::*;
        String::from(match value {
            Create => "create",
            Update => "update",
            Delete => "delete",
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i32>,
    pub object_type: ObjectType,
    pub object_id: i64,
    // Always "create," "update," or "delete."
    pub aspect_type: AspectType,
    // For activity update strava_events,
    //     keys can contain "title," "type,"
    //     and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone).
    // For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    pub updates: HashMap<String, String>,
    // The athlete's ID.
    pub owner_id: StravaId,
    // The push subscription ID that is receiving this event.
    pub subscription_id: i32,
    // The time that the event occurred.
    pub event_time: i64,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Event {}: {:?} {:?} {} at {}, owner:{}",
            self.id.unwrap_or(0),
            self.aspect_type,
            self.object_type,
            self.object_id,
            self.event_time,
            self.owner_id
        )
    }
}

impl Event {
    async fn delete(&self, store: &mut impl StravaStore) -> TbResult<()> {
        debug!("Deleting {self}");
        store.strava_event_delete(self.id).await
    }

    async fn setdate(&mut self, time: i64, store: &mut impl StravaStore) -> TbResult<()> {
        self.event_time = time;
        store.strava_event_set_time(self.id, self.event_time).await
    }

    // `#[async_recursion]` stamps `#[must_use]` on its generated boxed-future
    // wrapper, and clippy 1.99's `double_must_use` then flags that against the
    // already-must-use `Result` output. The attribute is macro-generated, so it
    // cannot be removed or given a message — allow the lint on the item instead.
    #[allow(clippy::double_must_use)]
    #[async_recursion]
    async fn rate_limit(
        self,
        user: &impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<QueueRead> {
        // rate limit event
        if self.object_id > get_time() {
            // still rate limited!
            return Ok(QueueRead::Limited {
                until: self.object_id,
            });
        }
        // remove stop event
        warn!("Starting hooks again");
        self.delete(store).await?;
        // get next event
        get_event(user, store).await
    }

    async fn process_activity(
        self,
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        match self.process_hook(user, store).await {
            Ok(summary) => Ok(summary),
            Err(err) => self.fail_event(err, store).await,
        }
    }

    /// Processes a Strava webhook event and performs the corresponding action.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to the Strava user associated with the webhook event.
    /// * `store` - A mutable reference to the database connection.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `Summary` map that summarizes the action performed.
    ///
    /// # Examples
    ///
    ///
    async fn process_hook(
        &self,
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let res = match self.aspect_type {
            AspectType::Create | AspectType::Update => {
                activity::upsert_activity(self.object_id, user, store).await?
            }
            AspectType::Delete => {
                match activity::delete_activity(self.object_id, user, store).await {
                    Err(Error::NotFound(_)) => {
                        warn!("Activity {} did not exist (yet)", self.object_id);
                        Summary::default()
                    }
                    res => res?,
                }
            }
        };
        self.delete(store).await?;
        Ok(res)
    }

    async fn sync(
        &mut self,
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        // let mut len = batch;
        let mut start = self.event_time;
        let mut summary = Summary::default();

        // while len == batch
        {
            let acts = next_activities(user, store, 25, start).await?;
            if acts.is_empty() {
                self.delete(store).await?;
            } else {
                trace!("processing sync event at {start}");
                for a in acts {
                    let act_id = a.id;
                    start = std::cmp::max(start, a.start_date.unix_timestamp());
                    match a.send_to_tb(user, store).await {
                        Ok(ps) => summary += ps,
                        // Rate limit, dead auth, or DB failure: bubble up to
                        // the classifier (`fail_event`) — the Stop keeps the
                        // event at the last good cursor, the disable drops the
                        // queue, the DB failure backs off in the loop.
                        Err(err)
                            if matches!(
                                err,
                                Error::TryAgain(_) | Error::NotAuth(_) | Error::DatabaseFailure(_)
                            ) =>
                        {
                            return Err(err);
                        }
                        // A permanently bad activity (404, parse): skip it and
                        // advance the cursor past it (spec #446 §4.6) —
                        // liveness > completeness. The admin sync endpoint
                        // re-covers the gap if the user cares.
                        Err(err) => {
                            warn!("skipping bad activity {act_id}: {err:#}");
                        }
                    }
                    self.setdate(start, store).await?;
                }
            }
        }

        Ok(summary)
    }

    async fn process_sync(
        mut self,
        user: &mut impl StravaSession,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        match self.sync(user, store).await {
            Ok(summary) => Ok(summary),
            Err(err) => self.fail_event(err, store).await,
        }
    }

    /// The failure semantics of a failed Strava message (spec #446 §4.6) —
    /// drop on permanent failure, no dead-letter table:
    ///
    /// - `TryAgain` (rate limited) → insert the global `Stop` and keep the
    ///   event queued: it is retried after the backoff.
    /// - `NotAuth` (401 after a token refresh) → disable the user's Strava
    ///   integration, which drops all their queued events: the data freezes
    ///   (stale but consistent), and no event-by-event 401s follow.
    /// - A DB failure → propagate: the executor loop rolls back (the event
    ///   stays queued) and applies its in-memory backoff.
    /// - Anything else is permanent (a 404 means the activity is gone; a parse
    ///   error will not heal): delete the event, log it, and move on — the
    ///   admin sync endpoint re-covers a gap if the user cares.
    ///
    /// Every arm returns `Ok` except the DB failure: the queue-side effects
    /// (the `Stop`, the disable, the delete) are committed by the loop's
    /// transaction, and the loop continues.
    async fn fail_event(&self, err: Error, store: &mut impl StravaStore) -> TbResult<Summary> {
        match err {
            Error::TryAgain(msg) => {
                warn!("Strava rate limited ({msg}); pausing all users for 15 minutes");
                insert_stop(store).await?;
                Ok(Summary::default())
            }
            Error::NotAuth(msg) => {
                warn!(
                    "Strava auth is dead ({msg}); disabling user {}",
                    self.owner_id
                );
                self.owner_id.disable(store).await?;
                Ok(Summary::default())
            }
            Error::DatabaseFailure(_) => Err(err),
            other => {
                warn!("dropping poison event {self}: {other:#}");
                self.delete(store).await?;
                Ok(Summary::default())
            }
        }
    }

    async fn process_user(&self, store: &mut impl StravaStore) -> TbResult<()> {
        debug!(
            "processing event user {}: {:?}",
            self.object_id, self.aspect_type,
        );

        match &self.aspect_type {
            AspectType::Update => {
                if self.updates.get("authorized") == Some(&String::from("false")) {
                    let res = self.owner_id.disable(store).await;
                    if let Err(err) = res {
                        error!("user disable returned: {err:#}")
                    }
                } else {
                    error!("Unknown updates {:?}", self.updates)
                }
            }
            x => error!("user event with {x:?}"),
        };

        Ok(())
    }
}

/// Inserts a new sync event into the database.
///
/// # Arguments
///
/// * `owner_id` - The ID of the Strava user associated with the sync event.
/// * `event_time` - The time of the sync event in Unix timestamp format.
/// * `store` - A mutable reference to the database connection.
///
/// # Returns
///
/// Returns a `Result` containing `()` if the operation was successful, or an `anyhow::Error` if an error occurred.
///
/// # Errors
///
/// This function may return an error if the `event_time` is greater than the current time.
///
/// # Examples
///
///
pub async fn insert_sync(
    owner_id: StravaId,
    event_time: i64,
    migrate: bool,
    store: &mut impl StravaStore,
) -> TbResult<()> {
    if event_time > get_time() {
        return Err(Error::BadRequest(format!("eventtime {event_time} > now!")));
    }
    let object_id = if migrate { 1 } else { 0 };
    let event = Event {
        owner_id,
        object_id,
        event_time,
        object_type: ObjectType::Sync,
        ..Default::default()
    };
    store.stravaevent_store(event).await
}

/// Inserts the global rate-limit `Stop` (spec #446 §4.6): the Strava rate
/// limit is app-wide, so one user's 429 pauses every user's drain. The
/// deadline is 15 minutes (the existing tuned value). If a `Stop` is already
/// queued, its deadline is **extended** rather than stacking rows — the
/// backoff must not compound: only `Stop`s carry `owner_id` 0, so the next
/// event for user 0 is the oldest `Stop`.
pub async fn insert_stop(store: &mut impl StravaStore) -> TbResult<()> {
    let stop = Event {
        object_type: ObjectType::Stop,
        object_id: get_time() + 900,
        ..Default::default()
    };
    if let Some(existing) = store
        .strava_event_get_next_for_user(StravaId::default())
        .await?
        && existing.object_type == ObjectType::Stop
    {
        existing.delete(store).await?;
    }
    store.stravaevent_store(stop).await
}

/// One outcome of a read of the `strava_events` queue (spec #446 §4.6):
/// what is interesting for the user right now, and when the queue is paused.
#[derive(Debug, Clone, PartialEq)]
pub enum QueueRead {
    /// The event to process (the read-side dedup already applied).
    Event(Event),
    /// The global rate-limit `Stop` is active until this unix time: the queue
    /// is paused, and the caller (the executor loop) sleeps until the deadline
    /// as an interruptible branch of its select — API writes are never delayed.
    Limited { until: i64 },
    /// The queue holds nothing interesting for this user (empty, or only an
    /// expired `Stop`, which the read deleted).
    Empty,
}

/// Read the next queued Strava event for the user, with the queue's two
/// read-side rules applied (ADR-0005, executable spec #446 §4.3):
///
/// - a `Stop` (the global rate-limit backoff) is checked against its expiry:
///   still limited → [`QueueRead::Limited`] (the event stays queued); expired
///   → deleted and the read recurses for the next event;
/// - only the latest event per object is interesting: older events for the
///   same object are deleted and the latest is returned.
///
/// The read runs inside the caller's transaction (the deletions above commit
/// or roll back with it). `pub` because the per-user executor loop (the
/// `tb_exec` crate) owns the "read next Strava event" step; reading the queue
/// stays a Strava-side concern, so the function stays here rather than being
/// inlined in the loop.
pub async fn get_event(
    user: &impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<QueueRead> {
    let event = store
        .strava_event_get_next_for_user(user.strava_id())
        .await?;
    let event = match event {
        Some(event) => event,
        None => return Ok(QueueRead::Empty),
    };
    if event.object_type == ObjectType::Stop {
        return event.rate_limit(user, store).await;
    }

    // Prevent unneeded calls to Strava
    // only the latest event for an object is interesting
    let mut list = store
        .strava_event_get_later(event.object_id, event.owner_id)
        .await?;
    let res = list.pop();

    if !list.is_empty() {
        debug!("Dropping {list:#?}");
        let values = list.into_iter().map(|l| l.id).collect::<Vec<_>>();
        store.strava_events_delete_batch(values).await?;
    }

    Ok(match res {
        Some(event) => QueueRead::Event(event),
        None => QueueRead::Empty,
    })
}

async fn next_activities(
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
    per_page: usize,
    start: i64,
) -> TbResult<Vec<StravaActivity>> {
    user.request_json(
        &format!("/activities?after={start}&per_page={per_page}"),
        store,
    )
    .await
}

pub async fn process(
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let read = get_event(user, store).await?;
    let event = match read {
        QueueRead::Event(event) => event,
        // The queue is empty, or paused by a global rate-limit `Stop` (the
        // executor loop sleeps until its deadline; nothing to process).
        QueueRead::Limited { .. } | QueueRead::Empty => return Ok(Summary::default()),
    };
    info!("Processing {event}");

    match event.object_type {
        ObjectType::Activity => event.process_activity(user, store).await,
        ObjectType::Sync => event.process_sync(user, store).await,
        _ => {
            warn!("skipping {event}");
            event.delete(store).await?;
            Ok(Summary::default())
        }
    }
}

pub async fn sync_users(
    user_id: Option<UserId>,
    time: i64,
    migrate: bool,
    store: &mut impl StravaStore,
) -> TbResult<()> {
    let users = match user_id {
        Some(id) => vec![store.stravauser_get_by_tbid(id).await?],
        None => store.stravausers_get_all().await?,
    };
    for user in users {
        if user.disabled() {
            warn!("user {} disabled, skipping", user.strava_id());
            continue;
        }
        info!("Adding sync for {:?} at {time}", user.strava_id());
        event::insert_sync(user.strava_id(), time, migrate, store).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestStravaSession, TestStravaStore, activity_json, strava_user};

    fn setup() -> (TestStravaStore, TestStravaSession) {
        let mut store = TestStravaStore::new();
        store.insert_user(strava_user(UserId::from(1), 42, true));
        let session = TestStravaSession::new(UserId::from(1), 42.into());
        (store, session)
    }

    fn in_event(
        object_type: &str,
        object_id: i64,
        aspect_type: &str,
        owner: i32,
        updates: serde_json::Map<String, serde_json::Value>,
    ) -> InEvent {
        serde_json::from_value(serde_json::json!({
            "object_type": object_type,
            "object_id": object_id,
            "aspect_type": aspect_type,
            "updates": updates,
            "owner_id": owner,
            "subscription_id": 1,
            "event_time": 100
        }))
        .unwrap()
    }

    fn activity_event() -> Event {
        Event {
            object_type: ObjectType::Activity,
            object_id: 10,
            aspect_type: AspectType::Create,
            owner_id: 42.into(),
            subscription_id: 1,
            event_time: 100,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn into_event_maps_fields() -> TbResult<()> {
        let (mut store, _) = setup();
        let event = in_event("activity", 10, "create", 42, serde_json::Map::new())
            .into_event(&mut store)
            .await?;
        assert!(event.id.is_none());
        assert_eq!(event.object_type, ObjectType::Activity);
        assert_eq!(event.object_id, 10);
        assert_eq!(event.aspect_type, AspectType::Create);
        assert_eq!(event.owner_id, 42.into());
        assert_eq!(event.subscription_id, 1);
        assert_eq!(event.event_time, 100);
        Ok(())
    }

    #[tokio::test]
    async fn into_event_rejects_unknown_owner() {
        let (mut store, _) = setup();
        let res = in_event("activity", 10, "create", 99, serde_json::Map::new())
            .into_event(&mut store)
            .await;
        assert!(matches!(res, Err(Error::BadRequest(_))));
    }

    #[tokio::test]
    async fn into_event_rejects_unknown_types() {
        let (mut store, _) = setup();
        let bad_object = in_event("flying", 10, "create", 42, serde_json::Map::new());
        assert!(matches!(
            bad_object.into_event(&mut store).await,
            Err(Error::BadRequest(_))
        ));
        let bad_aspect = in_event("activity", 10, "explode", 42, serde_json::Map::new());
        assert!(matches!(
            bad_aspect.into_event(&mut store).await,
            Err(Error::BadRequest(_))
        ));
    }

    #[tokio::test]
    async fn accept_stores_activity_event() -> TbResult<()> {
        let (mut store, _) = setup();
        assert!(
            in_event("activity", 10, "create", 42, serde_json::Map::new())
                .accept(&mut store)
                .await?
                .is_some()
        );
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.events[0].object_id, 10);
        assert!(store.events[0].id.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn accept_athlete_deauth_disables_user() -> TbResult<()> {
        let (mut store, _) = setup();
        store
            .stravaevent_store(Event {
                owner_id: 42.into(),
                object_type: ObjectType::Activity,
                object_id: 10,
                aspect_type: AspectType::Create,
                ..Default::default()
            })
            .await?;
        let mut updates = serde_json::Map::new();
        updates.insert("authorized".into(), serde_json::json!("false"));
        assert!(
            in_event("athlete", 42, "update", 42, updates)
                .accept(&mut store)
                .await?
                .is_none()
        );
        assert_eq!(store.event_count(), 0);
        let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
        assert!(user.disabled());
        Ok(())
    }

    #[tokio::test]
    async fn insert_sync_stores_event() -> TbResult<()> {
        let (mut store, _) = setup();
        insert_sync(42.into(), 100, true, &mut store).await?;
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.events[0].object_type, ObjectType::Sync);
        assert_eq!(store.events[0].object_id, 1);
        Ok(())
    }

    #[tokio::test]
    async fn insert_sync_rejects_future() {
        let (mut store, _) = setup();
        let res = insert_sync(42.into(), get_time() + 1000, false, &mut store).await;
        assert!(matches!(res, Err(Error::BadRequest(_))));
        assert_eq!(store.event_count(), 0);
    }

    #[tokio::test]
    async fn insert_stop_schedules_future() -> TbResult<()> {
        let (mut store, _) = setup();
        insert_stop(&mut store).await?;
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.events[0].object_type, ObjectType::Stop);
        assert!(store.events[0].object_id > get_time());
        Ok(())
    }

    #[tokio::test]
    async fn insert_stop_extends_existing() -> TbResult<()> {
        // A second 429 while a Stop is queued extends the deadline rather
        // than stacking rows (spec #446 §4.6): the backoff must not compound.
        let (mut store, _) = setup();
        insert_stop(&mut store).await?;
        let first_until = store.events[0].object_id;
        insert_stop(&mut store).await?;
        assert_eq!(
            store.event_count(),
            1,
            "the second Stop extends, it does not stack"
        );
        let stop = &store.events[0];
        assert_eq!(stop.object_type, ObjectType::Stop);
        assert!(
            stop.object_id >= first_until,
            "the deadline is never shortened"
        );
        assert!(
            stop.object_id > get_time(),
            "the extended Stop is still active"
        );
        Ok(())
    }

    #[tokio::test]
    async fn get_event_returns_latest_and_drops_older() -> TbResult<()> {
        let (mut store, session) = setup();
        let mut first = activity_event();
        first.aspect_type = AspectType::Update;
        first.event_time = 50;
        store.stravaevent_store(first).await?;
        store.stravaevent_store(activity_event()).await?;
        let event = match get_event(&session, &mut store).await? {
            QueueRead::Event(event) => event,
            other => panic!("the latest event is returned, got {other:?}"),
        };
        assert_eq!(event.aspect_type, AspectType::Create);
        assert_eq!(store.event_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn get_event_ignores_other_owners() -> TbResult<()> {
        let (mut store, session) = setup();
        store
            .stravaevent_store(Event {
                owner_id: 43.into(),
                ..activity_event()
            })
            .await?;
        assert_eq!(get_event(&session, &mut store).await?, QueueRead::Empty);
        Ok(())
    }

    #[tokio::test]
    async fn get_event_active_stop_reports_deadline() -> TbResult<()> {
        // An unexpired global Stop pauses the queue: the read reports the
        // deadline (the executor loop sleeps until it) and consumes nothing.
        let (mut store, session) = setup();
        let until = get_time() + 500;
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Stop,
                object_id: until,
                ..Default::default()
            })
            .await?;
        store.stravaevent_store(activity_event()).await?;
        let res = get_event(&session, &mut store).await?;
        assert_eq!(res, QueueRead::Limited { until });
        assert_eq!(store.event_count(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn get_event_expired_stop_deletes_and_returns_next() -> TbResult<()> {
        // An expired Stop is deleted by the read, which recurses for the next
        // event (the backoff is over).
        let (mut store, session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Stop,
                object_id: 100,
                ..Default::default()
            })
            .await?;
        store.stravaevent_store(activity_event()).await?;
        let res = get_event(&session, &mut store).await?;
        assert!(matches!(res, QueueRead::Event(ref e) if e.object_id == 10));
        assert_eq!(store.event_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn rate_limit_active_reports_deadline() -> TbResult<()> {
        let (mut store, session) = setup();
        let until = get_time() + 500;
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Stop,
                object_id: until,
                ..Default::default()
            })
            .await?;
        let event = store.events[0].clone();
        let res = event.rate_limit(&session, &mut store).await?;
        assert_eq!(res, QueueRead::Limited { until });
        assert_eq!(store.event_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn rate_limit_expired_removes_stop() -> TbResult<()> {
        let (mut store, session) = setup();
        let stop = Event {
            object_type: ObjectType::Stop,
            object_id: 100,
            ..Default::default()
        };
        store.stravaevent_store(stop).await?;
        let event = store.events[0].clone();
        let res = event.rate_limit(&session, &mut store).await?;
        assert_eq!(res, QueueRead::Empty);
        assert_eq!(store.event_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn process_no_events() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        Ok(())
    }

    #[tokio::test]
    async fn process_activity_create_imports() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store.stravaevent_store(activity_event()).await?;
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(summary.activities.keys().next(), Some(&ActivityId::new(10)));
        assert_eq!(store.event_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn process_activity_delete_missing_ignores() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                aspect_type: AspectType::Delete,
                ..activity_event()
            })
            .await?;
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn process_try_again_inserts_stop() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store.stravaevent_store(activity_event()).await?;
        session.queue_error("/activities/10", Error::TryAgain("rate limit"));
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 2);
        let stop = store
            .events
            .iter()
            .find(|e| e.object_type == ObjectType::Stop)
            .unwrap();
        assert!(stop.object_id > get_time());
        Ok(())
    }

    #[tokio::test]
    async fn process_activity_nontransient_drops_event() -> TbResult<()> {
        // A permanently bad activity (parse error, a 404's domain shape) is
        // dropped from the queue and logged — it is committed, so the event
        // is gone and the queue cannot wedge on it (spec #446 §4.6).
        let (mut store, mut session) = setup();
        store.stravaevent_store(activity_event()).await?;
        session.queue_error(
            "/activities/10",
            Error::AnyFailure(anyhow::anyhow!("parse boom")),
        );
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 0, "the poison event is deleted");
        Ok(())
    }

    #[tokio::test]
    async fn process_activity_not_auth_disables_user() -> TbResult<()> {
        // A 401 after a token refresh disables the user's Strava integration
        // and drops all their queued events (spec #446 §4.6): the data freezes
        // stale-but-consistent; no event-by-event 401s follow.
        let (mut store, mut session) = setup();
        store.stravaevent_store(activity_event()).await?;
        session.queue_error("/activities/10", Error::NotAuth(String::from("token dead")));
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(
            store.event_count(),
            0,
            "the user's queued events are dropped"
        );
        let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
        assert!(user.disabled(), "the StravaId is disabled");
        Ok(())
    }

    #[tokio::test]
    async fn process_sync_not_auth_disables_user() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Sync,
                owner_id: 42.into(),
                ..Default::default()
            })
            .await?;
        session.queue_error(
            "/activities?after=0&per_page=25",
            Error::NotAuth(String::from("token dead")),
        );
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 0);
        let user = store.stravauser_get_by_tbid(UserId::from(1)).await?;
        assert!(user.disabled());
        Ok(())
    }

    #[tokio::test]
    async fn process_sync_empty_list_removes_event() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Sync,
                owner_id: 42.into(),
                ..Default::default()
            })
            .await?;
        session.queue("/activities?after=0&per_page=25", "[]");
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn process_sync_imports_activities() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Sync,
                owner_id: 42.into(),
                ..Default::default()
            })
            .await?;
        session.queue(
            "/activities?after=0&per_page=25",
            &format!("[{}]", activity_json(10, "Ride", None)),
        );
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary.activities.len(), 1);
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.events[0].event_time, 1767348000);
        let acts = ActivityStore::get_all(&mut store.mem, &UserId::from(1)).await?;
        assert_eq!(acts.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn process_sync_bad_activity_skips_and_advances_cursor() -> TbResult<()> {
        // A permanently bad activity in a batch is skipped and the cursor
        // advances past it (spec #446 §4.6) — liveness > completeness: one bad
        // activity must not wedge the queue in an infinite retry loop. The
        // event stays queued at the advanced cursor (one page per wake); the
        // admin sync endpoint re-covers the gap if the user cares.
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Sync,
                owner_id: 42.into(),
                ..Default::default()
            })
            .await?;
        session.queue(
            "/activities?after=0&per_page=25",
            &format!(
                "[{}, {}]",
                activity_json(10, "Ride", None),
                activity_json(11, "Ride", Some("g11"))
                    .replace("2026-01-02T10:00:00Z", "2026-01-03T10:00:00Z")
            ),
        );
        session.queue("/activities/10", &activity_json(10, "Ride", None));
        session.queue_error(
            "/gear/g11",
            Error::AnyFailure(anyhow::anyhow!("parse boom")),
        );
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary.activities.len(), 1, "the good activity imports");
        assert_eq!(store.event_count(), 1, "the sync event stays queued");
        assert_eq!(
            store.events[0].event_time, 1767434400,
            "the cursor advances past the bad activity (2026-01-03T10:00:00Z)"
        );
        Ok(())
    }

    #[tokio::test]
    async fn process_sync_try_again_keeps_event_and_stops() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Sync,
                owner_id: 42.into(),
                ..Default::default()
            })
            .await?;
        session.queue_error("/activities?after=0&per_page=25", Error::TryAgain("nope"));
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 2);
        assert!(
            store
                .events
                .iter()
                .any(|e| e.object_type == ObjectType::Stop)
        );
        Ok(())
    }

    #[tokio::test]
    async fn process_skips_athlete_event() -> TbResult<()> {
        let (mut store, mut session) = setup();
        store
            .stravaevent_store(Event {
                object_type: ObjectType::Athlete,
                object_id: 42,
                ..Default::default()
            })
            .await?;
        let summary = process(&mut session, &mut store).await?;
        assert_eq!(summary, Summary::default());
        assert_eq!(store.event_count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn sync_users_skips_disabled() -> TbResult<()> {
        let mut store = TestStravaStore::new();
        UserStore::create(&mut store.mem, "A", "User", &None).await?;
        UserStore::create(&mut store.mem, "B", "User", &None).await?;
        store.insert_user(strava_user(UserId::from(1), 42, true));
        store.insert_user(strava_user(UserId::from(2), 43, false));
        sync_users(None, 100, false, &mut store).await?;
        assert_eq!(store.event_count(), 1);
        assert_eq!(store.events[0].owner_id, 42.into());
        Ok(())
    }

    #[tokio::test]
    async fn sync_users_single() -> TbResult<()> {
        let (mut store, _) = setup();
        sync_users(Some(UserId::from(1)), 100, false, &mut store).await?;
        assert_eq!(store.event_count(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn sync_users_unknown_user() {
        let (mut store, _) = setup();
        let res = sync_users(Some(UserId::from(99)), 100, false, &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
    }
}
