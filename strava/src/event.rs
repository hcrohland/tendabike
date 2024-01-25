use async_recursion::async_recursion;
use std::collections::HashMap;

use crate::activity::StravaActivity;
use crate::*;

#[derive(Debug, Serialize, Deserialize)]
/// A struct representing an incoming Strava event.
pub struct InEvent {
    object_type: String,
    object_id: i64,
    // Always "create," "update," or "delete."
    aspect_type: String,
    // hash 	For activity update strava_events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
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
    pub async fn to_event(self, conn: &mut impl StravaStore) -> TbResult<Event> {
        let objects = ["activity", "athlete"];
        let aspects = ["create", "update", "delete"];

        if !(objects.contains(&self.object_type.as_str())
            && aspects.contains(&self.aspect_type.as_str()))
        {
            return Err(Error::BadRequest(format!(
                "unknown event received: {:?}",
                self
            )));
        };

        if StravaId::read(&self.owner_id.into(), conn).await?.is_none() {
            return Err(Error::BadRequest(format!(
                "Unknown event owner received: {:?}",
                self
            )));
        }

        Ok(Event {
            id: None,
            object_type: self.object_type,
            object_id: self.object_id,
            aspect_type: self.aspect_type,
            owner_id: self.owner_id.into(),
            subscription_id: self.subscription_id,
            event_time: self.event_time,
            updates: serde_json::to_string(&self.updates).unwrap_or_else(|e| format!("{:?}", e)),
        })
    }

    pub async fn accept(self, conn: &mut impl StravaStore) -> TbResult<()> {
        let event = self.to_event(conn).await?;
        conn.stravaevent_store(event).await?;
        Ok(())
    }
}
#[derive(Debug, Default, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = crate::schema::strava_events)]
pub struct Event {
    id: Option<i32>,
    pub object_type: String,
    pub object_id: i64,
    // Always "create," "update," or "delete."
    pub aspect_type: String,
    // hash 	For activity update events, keys can contain "title," "type," and "private," which is always "true" (activity visibility set to Only You) or "false" (activity visibility set to Followers Only or Everyone). For app deauthorization events, there is always an "authorized" : "false" key-value pair.
    updates: String,
    // The athlete's ID.
    owner_id: StravaId,
    // The push subscription ID that is receiving this event.
    subscription_id: i32,
    // The time that the event occurred.
    pub event_time: i64,
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Event {}: {} {} {} at {}, owner:{}",
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
    async fn delete(&self, conn: &mut impl StravaStore) -> TbResult<()> {
        debug!("Deleting {}", self);
        conn.strava_event_delete(self.id).await
    }

    async fn setdate(&mut self, time: i64, conn: &mut impl StravaStore) -> TbResult<()> {
        self.event_time = time;
        conn.strava_event_set_time(self.id, self.event_time).await
    }

    #[async_recursion]
    async fn rate_limit(
        self,
        user: &impl StravaPerson,
        conn: &mut impl StravaStore,
    ) -> TbResult<Option<Self>> {
        // rate limit event
        if self.event_time > get_time() {
            // still rate limited!
            return Ok(None);
        }
        // remove stop event
        warn!("Starting hooks again");
        self.delete(conn).await?;
        // get next event
        get_event(user, conn).await
    }

    async fn process_activity(
        self,
        user: &mut impl StravaPerson,
        conn: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let summary = self.process_hook(user, conn).await;
        let summary = match summary {
            Ok(x) => Ok(x),
            Err(e) => check_try_again(e, conn).await,
        };
        match summary {
            Ok(res) => Ok(res),
            Err(err) => {
                self.delete(conn).await?;
                Err(err)
            }
        }
    }

    /// Processes a Strava webhook event and performs the corresponding action.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to the Strava user associated with the webhook event.
    /// * `conn` - A mutable reference to the database connection.
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
        conn: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let res = match self.aspect_type.as_str() {
            "create" | "update" => activity::upsert_activity(self.object_id, user, conn).await?,
            "delete" => activity::delete_activity(self.object_id, user, conn).await?,
            _ => {
                warn!("Skipping unknown aspect_type {:?}", self);
                Summary::default()
            }
        };
        self.delete(conn).await?;
        Ok(res)
    }

    async fn sync(
        mut self,
        user: &mut impl StravaPerson,
        conn: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        // let mut len = batch;
        let mut start = self.event_time;
        let mut summary = Summary::default();

        // while len == batch
        {
            let acts = next_activities(user, conn, 10, start).await?;
            if acts.is_empty() {
                self.delete(conn).await?;
            } else {
                for a in acts {
                    start = std::cmp::max(start, a.start_date.unix_timestamp());
                    trace!("processing sync event at {}", start);
                    let ps = a.send_to_tb(user, conn).await?;
                    self.setdate(start, conn).await?;
                    summary = summary.merge(ps);
                }
            }
        }

        Ok(summary)
    }

    async fn process_sync(
        self,
        user: &mut impl StravaPerson,
        conn: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        let summary = self.sync(user, conn).await;
        if let Err(err) = summary {
            return check_try_again(err, conn).await;
        }
        summary
    }
}

/// Inserts a new sync event into the database.
///
/// # Arguments
///
/// * `owner_id` - The ID of the Strava user associated with the sync event.
/// * `event_time` - The time of the sync event in Unix timestamp format.
/// * `conn` - A mutable reference to the database connection.
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
    conn: &mut impl StravaStore,
) -> TbResult<()> {
    if event_time > get_time() {
        return Err(Error::BadRequest(format!(
            "eventtime {} > now!",
            event_time
        )));
    }
    let event = Event {
        owner_id,
        event_time,
        object_type: "sync".to_string(),
        ..Default::default()
    };
    conn.stravaevent_store(event).await
}

pub async fn insert_stop(conn: &mut impl StravaStore) -> TbResult<()> {
    let e = Event {
        object_type: "stop".to_string(),
        object_id: get_time() + 900,
        ..Default::default()
    };
    conn.stravaevent_store(e).await
}

async fn get_event(
    user: &impl StravaPerson,
    conn: &mut impl StravaStore,
) -> TbResult<Option<Event>> {
    let event = conn.strava_event_get_next_for_user(user).await?;
    let event = match event {
        Some(event) => event,
        None => return Ok(None),
    };
    if event.object_type.as_str() == "stop" {
        return event.rate_limit(user, conn).await;
    }

    // Prevent unneeded calls to Strava
    // only the latest event for an object is interesting
    let mut list = conn
        .strava_event_get_later(event.object_id, event.owner_id)
        .await?;
    let res = list.pop();

    if !list.is_empty() {
        debug!("Dropping {:#?}", list);
        let values = list.into_iter().map(|l| l.id).collect::<Vec<_>>();
        conn.strava_events_delete_batch(values).await?;
    }

    Ok(res)
}

async fn check_try_again(err: tb_domain::Error, conn: &mut impl StravaStore) -> TbResult<Summary> {
    // Keep events for temporary failure - delete others
    match err {
        Error::TryAgain(_) => {
            warn!("Stopping hooks for 15 minutes {:?}", err);
            insert_stop(conn).await?;
            Ok(Summary::default())
        }
        _ => Err(err),
    }
}

async fn next_activities(
    user: &mut impl StravaPerson,
    conn: &mut impl StravaStore,
    per_page: usize,
    start: i64,
) -> TbResult<Vec<StravaActivity>> {
    user.request_json(
        &format!("/activities?after={}&per_page={}", start, per_page),
        conn,
    )
    .await
}

pub async fn process(
    user: &mut impl StravaPerson,
    conn: &mut impl StravaStore,
) -> TbResult<Summary> {
    let event = get_event(user, conn).await?;
    if event.is_none() {
        return Ok(Summary::default());
    };
    let event = event.unwrap();
    info!("Processing {}", event);

    match event.object_type.as_str() {
        "activity" => event.process_activity(user, conn).await,
        "sync" => event.process_sync(user, conn).await,
        // "athlete" => process_user(e, user),
        _ => {
            warn!("skipping {}", event);
            event.delete(conn).await?;
            Ok(Summary::default())
        }
    }
}

pub async fn sync_users(
    user_id: Option<UserId>,
    time: i64,
    conn: &mut impl StravaStore,
) -> TbResult<()> {
    let users = match user_id {
        Some(id) => vec![conn.stravauser_get_by_tbid(id).await?],
        None => conn.stravausers_get_all().await?,
    };
    for user in users {
        if user.disabled() {
            warn!("user {} disabled, skipping", user.strava_id());
            continue;
        }
        info!("Adding sync for {:?} at {}", user.strava_id(), time);
        event::insert_sync(user.strava_id(), time, conn).await?;
    }
    Ok(())
}
