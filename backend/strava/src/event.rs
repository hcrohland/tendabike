use async_recursion::async_recursion;
use async_session::log::error;
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

    pub async fn accept(self, store: &mut impl StravaStore) -> TbResult<()> {
        let event = self.into_event(store).await?;
        if event.object_type == ObjectType::Athlete {
            event.process_user(store).await?;
        } else {
            store.stravaevent_store(event).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
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

    #[async_recursion]
    async fn rate_limit(
        self,
        user: &impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Option<Self>> {
        // rate limit event
        if self.object_id > get_time() {
            // still rate limited!
            return Ok(None);
        }
        // remove stop event
        warn!("Starting hooks again");
        self.delete(store).await?;
        // get next event
        get_event(user, store).await
    }

    async fn process_activity(
        self,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let summary = self.process_hook(user, store).await;
        let summary = match summary {
            Ok(x) => Ok(x),
            Err(e) => check_try_again(e, store).await,
        };
        match summary {
            Ok(res) => Ok(res),
            Err(err) => {
                self.delete(store).await?;
                Err(err)
            }
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
    /// Returns a `Result` containing a `Summary` struct that summarizes the action performed.
    ///
    /// # Examples
    ///
    ///
    async fn process_hook(
        &self,
        user: &mut impl StravaPerson,
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
        mut self,
        user: &mut impl StravaPerson,
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
                    start = std::cmp::max(start, a.start_date.unix_timestamp());
                    let ps = a.send_to_tb(user, store).await?;
                    self.setdate(start, store).await?;
                    summary = summary + ps;
                }
            }
        }

        Ok(summary)
    }

    async fn process_sync(
        self,
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let summary = self.sync(user, store).await;
        if let Err(err) = summary {
            return check_try_again(err, store).await;
        }
        summary
    }

    async fn process_user(&self, store: &mut impl StravaStore) -> TbResult<()> {
        debug!(
            "processing event user {}: {:?}",
            &self.object_id, &self.aspect_type,
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

pub async fn insert_stop(store: &mut impl StravaStore) -> TbResult<()> {
    let e = Event {
        object_type: ObjectType::Stop,
        object_id: get_time() + 900,
        ..Default::default()
    };
    store.stravaevent_store(e).await
}

async fn get_event(
    user: &impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Option<Event>> {
    let event = store
        .strava_event_get_next_for_user(user.strava_id())
        .await?;
    let event = match event {
        Some(event) => event,
        None => return Ok(None),
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

    Ok(res)
}

async fn check_try_again(err: tb_domain::Error, store: &mut impl StravaStore) -> TbResult<Summary> {
    // Keep events for temporary failure - delete others
    match err {
        Error::TryAgain(_) => {
            warn!("Stopping hooks for 15 minutes {err:?}");
            insert_stop(store).await?;
            Ok(Summary::default())
        }
        _ => Err(err),
    }
}

async fn next_activities(
    user: &mut impl StravaPerson,
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
    user: &mut impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<Summary> {
    let event = get_event(user, store).await?;
    if event.is_none() {
        return Ok(Summary::default());
    };
    let event = event.unwrap();
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
