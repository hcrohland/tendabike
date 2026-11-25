use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::activity::StravaActivity;
use super::http::request_json;
use super::session::get_strava_id;
#[cfg(test)]
use super::{ActivityId, MockStravaDataStore};
use super::{Error, StravaId, StravaSpec, Summary, TbResult, UserId, activity, get_time};
use crate::EntityFacade;
use crate::lane::QueueRead;
use crate::store::Store;

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
    owner_id: StravaId,
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
    async fn into_event(self, store: &mut impl Store<StravaSpec>) -> TbResult<Event> {
        if StravaId::read(&self.owner_id, store).await?.is_none() {
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
    /// the caller can wake that user), or `None` for an athlete
    /// (de)authorization (nothing was queued, so there is nothing to wake).
    pub async fn accept(self, store: &mut impl Store<StravaSpec>) -> TbResult<Option<UserId>> {
        let user_id = StravaId::read(&self.owner_id, store)
            .await?
            .map(|user| user.tendabike_id)
            .ok_or_else(|| Error::BadRequest(format!("Unknown event owner received: {self:?}")))?;
        let event = self.into_event(store).await?;
        if event.object_type == ObjectType::Athlete {
            event.process_user(store).await?;
            Ok(None)
        } else {
            store.event_create(event).await?;
            Ok(Some(user_id))
        }
    }
}

/// The webhook intake (ADR-0014): the provider's webhook work behind the
/// seam — the envelope parse of the raw body, then the accept (the queue
/// insert, or the athlete-deauth branch). `body` is the raw webhook body the
/// presentation layer forwards (no driver wire type crosses the seam).
/// Returns the owner's TendaBike [`UserId`] when a row was queued (the
/// caller's wake), or `None` for an athlete (de)authorization.
pub async fn accept_event(
    body: &str,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<Option<UserId>> {
    let event: InEvent = serde_json::from_str(body)
        .map_err(|e| Error::BadRequest(format!("invalid strava webhook envelope: {e}")))?;
    trace!("Received {event:#?}");
    event.accept(store).await
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub enum ObjectType {
    #[default]
    Activity,
    Athlete,
    Sync,
}

impl TryFrom<String> for ObjectType {
    type Error = crate::facade::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        use ObjectType::*;
        Ok(match value.as_str() {
            "activity" => Activity,
            "athlete" => Athlete,
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
    type Error = crate::facade::Error;

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
    async fn delete(&self, store: &mut impl Store<StravaSpec>) -> TbResult<()> {
        debug!("Deleting {self}");
        store.event_delete(self.id).await
    }

    async fn setdate(&mut self, time: i64, store: &mut impl Store<StravaSpec>) -> TbResult<()> {
        self.event_time = time;
        store.event_set_time(self.id, self.event_time).await
    }

    async fn process_activity<D: EntityFacade<S>, S: Store<StravaSpec>>(
        &self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        match self.process_hook(user_id, facade, store).await {
            Ok(summary) => Ok(summary),
            // The kit's failure policy (spec #446 §4.6): the disable and the
            // drop commit with the loop's transaction.
            Err(err) => crate::queue::fail_event(self, err, store).await,
        }
    }

    /// Processes a Strava webhook event and performs the corresponding action.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user associated with the webhook event.
    /// * `store` - A mutable reference to the database connection.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a `Summary` map that summarizes the action performed.
    ///
    /// # Examples
    ///
    ///
    async fn process_hook<D: EntityFacade<S>, S: Store<StravaSpec>>(
        &self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        let res = match self.aspect_type {
            AspectType::Create | AspectType::Update => {
                activity::upsert_activity(self.object_id, user_id, facade, store).await?
            }
            AspectType::Delete => {
                match activity::delete_activity(self.object_id, user_id, facade, store).await {
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

    async fn sync<D: EntityFacade<S>, S: Store<StravaSpec>>(
        &mut self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        // let mut len = batch;
        let mut start = self.event_time;
        let mut summary = Summary::default();

        // while len == batch
        {
            let acts = next_activities(user_id, store, 25, start).await?;
            if acts.is_empty() {
                self.delete(store).await?;
            } else {
                trace!("processing sync event at {start}");
                for a in acts {
                    let act_id = a.id;
                    start = std::cmp::max(start, a.start_date.unix_timestamp());
                    match a.send_to_tb(user_id, facade, store).await {
                        Ok(ps) => summary += ps,
                        // Rate limit, dead auth, or DB failure: bubble up to
                        // the failure policy (`fail_event`) — the Stop keeps
                        // the event at the last good cursor, the disable drops
                        // the queue, the DB failure backs off in the loop.
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

    async fn process_sync<D: EntityFacade<S>, S: Store<StravaSpec>>(
        mut self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Summary> {
        match self.sync(user_id, facade, store).await {
            Ok(summary) => Ok(summary),
            // The kit's failure policy (spec #446 §4.6): the Stop keeps the
            // event at the last good cursor, the disable drops the queue, the
            // DB failure backs off in the loop.
            Err(err) => crate::queue::fail_event(&self, err, store).await,
        }
    }

    async fn process_user(&self, store: &mut impl Store<StravaSpec>) -> TbResult<()> {
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
    user_id: UserId,
    event_time: i64,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<()> {
    if event_time > get_time() {
        return Err(Error::BadRequest(format!("eventtime {event_time} > now!")));
    }
    let owner_id = get_strava_id(user_id, store).await?;
    let event = Event {
        owner_id,
        object_id: 0,
        event_time,
        object_type: ObjectType::Sync,
        ..Default::default()
    };
    store.event_create(event).await
}

/// Whether `event` is the admin sync for `user_id` at `event_time` (spec
/// #558 §6.4): the consumed event is ours when its identity — a `Sync` for the
/// user's `strava_id` at that time — matches the sync we queued.
pub async fn is_sync_for_user(
    event: &Event,
    user_id: UserId,
    event_time: i64,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<bool> {
    let strava_id = get_strava_id(user_id, store).await?;
    Ok(event.object_type == ObjectType::Sync
        && event.owner_id == strava_id
        && event.event_time == event_time)
}

/// Whether `user_id`'s queue still holds the sync at `event_time` (the admin
/// sync's fallback completion check, spec #4.6): every sync is queued with
/// `object_id` 0, so the user's `object_id` 0 events are exactly their syncs.
pub async fn sync_queued_for_user(
    user_id: UserId,
    event_time: i64,
    store: &mut impl Store<StravaSpec>,
) -> TbResult<bool> {
    let strava_id = get_strava_id(user_id, store).await?;
    let syncs = store.event_get_later(0, strava_id).await?;
    Ok(syncs.iter().any(|event| event.event_time == event_time))
}

async fn next_activities(
    user_id: UserId,
    store: &mut impl Store<StravaSpec>,
    per_page: usize,
    start: i64,
) -> TbResult<Vec<StravaActivity>> {
    request_json(
        user_id,
        &format!("/activities?after={start}&per_page={per_page}"),
        store,
    )
    .await
}

/// Process the user's next queued Strava event (the lane's `process`): read
/// the queue with the kit's read-side rules ([`crate::queue::get_event`]) and
/// handle that one event. One event per call — the queue's read and failure
/// policy are the kit's, generic over the marker (spec #1029).
pub(crate) async fn process<D: EntityFacade<S>, S: Store<StravaSpec>>(
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Summary> {
    let strava_id = get_strava_id(user_id, store).await?;
    let read = crate::queue::get_event(strava_id, store).await?;
    let event = match read {
        QueueRead::Event(event) => event,
        // The queue is empty, or paused by a global rate-limit `Stop` (the
        // caller sleeps until its deadline; nothing to process).
        QueueRead::Limited { .. } | QueueRead::Empty => return Ok(Summary::default()),
    };
    info!("Processing {event}");

    match event.object_type {
        ObjectType::Activity => event.process_activity(user_id, facade, store).await,
        ObjectType::Sync => event.process_sync(user_id, facade, store).await,
        _ => {
            warn!("skipping {event}");
            event.delete(store).await?;
            Ok(Summary::default())
        }
    }
}

#[cfg(test)]
mod event_tests;
