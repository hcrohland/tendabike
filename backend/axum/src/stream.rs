//! The per-user executor registry and the SSE stream endpoint (ADR-0005,
//! executable spec #446 §4.4, §4.5, §6.1).
//!
//! The web layer owns the SSE streams; the per-user executor task (the
//! `tb_exec` loop) is spawned on demand (the first SSE connect **or** the
//! first API write) and reaped on idle. The [`Registry`] tracks the live
//! executor per [`UserId`]: a second connect (or write) finds the running
//! task rather than spawning a duplicate, and a reaped task is respawned on
//! the next demand — the stream re-registers on the respawned executor's
//! frame channel.
//!
//! Each SSE frame is the completed message's [`Summary`] (the map JSON, spec
//! §2) — no envelope, no type discriminator, no sequence number. While the
//! executor is reaped (idle), the stream stays alive on heartbeat comments
//! (`: ping`), invisible to the client's `onmessage`.
//!
//! The executor's fate is shared through a `watch` channel (a `JoinHandle` is
//! not `Clone`, so it cannot be handed to both the registry and the stream
//! tasks): the spawned task records [`ExecutorStatus::Reaped`] on a clean
//! idle exit and [`ExecutorStatus::Dead`] on a lifecycle error or panic. A
//! clone of the frame sender is kept alive in the task until *after* that
//! record is written, so a stream that sees the frame channel close already
//! sees the settled status.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::unfold;
use http::StatusCode;
use log::{error, info};
use tokio::sync::{Mutex, Notify, broadcast, watch};

use tb_domain::{ApiWrite, DescendReport, Error, Session, Summary, TbResult, UserId, WriteOutcome};
use tb_exec::{ApiWriteRequest, ApiWriteSender, Txn, TxnSource, api_write_channel, run};
use tb_strava::event::{Event as StravaEvent, ObjectType};
use tb_strava::{StravaId, StravaStore};

use crate::appstate::AppState;
use crate::strava::RequestSession;

/// The loop's idle window before it reaps while streams are still attached
/// (spec §4.4): long enough that an active tab's executor is not churned by a
/// lull in writes, short enough that an abandoned tab's executor is reclaimed.
pub(crate) const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// The SSE heartbeat cadence (spec §4.5): a `: ping` comment keeps the
/// connection alive (and proxies) while the executor is reaped.
pub(crate) const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// The per-user frame channel's capacity: a burst of frames for a lagged
/// stream is dropped (the client's reconnect + full refresh covers it), so a
/// modest bound is enough.
const BROADCAST_CAPACITY: usize = 64;

/// The executor task's settled fate, shared with its streams (a `JoinHandle`
/// is not `Clone`, so it cannot be handed to both the registry and the stream
/// tasks). `Running` is the initial value; the task records `Reaped` on a
/// clean idle exit and `Dead` on a lifecycle error, and a watchdog records
/// `Dead` if the task panics (a panic skips the task's own record).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutorStatus {
    /// The loop is running (or has not yet recorded its exit).
    Running,
    /// The loop exited cleanly on idle (`run` returned `Ok(())`): the stream
    /// re-subscribes (respawning the executor) and keeps heartbeating.
    Reaped,
    /// The loop ended with a lifecycle error, or the task panicked: the
    /// stream closes so the client reconnects and respawns (spec §4.5).
    Dead,
}

/// One live per-user executor: the shared fate channel, the send half of the
/// user's API-write channel (the write handlers enqueue here, spec §6.2),
/// the SSE frame sink the stream tasks subscribe to, the consumed-
/// Strava-event channel the admin sync awaits (spec §6.4), the in-memory
/// wake signal the webhook fires after queueing a Strava event (spec
/// §4.4/§4.6), and the task's abort handle (the registry's `stop_all` tears
/// the executor down — rescan, issue #446). The `AbortHandle` (not the
/// `JoinHandle`) is stored: the watchdog keeps the original handle to
/// `await` and record `Dead` on a panic **or** the abort `JoinError`.
#[derive(Clone)]
struct Executor {
    status: watch::Receiver<ExecutorStatus>,
    /// The write handlers' door into this user's loop (spec §6.2): enqueue
    /// an `ApiWrite` here and await its oneshot.
    writer: ApiWriteSender,
    frames: broadcast::Sender<Summary>,
    /// Every Strava event the loop consumed (spec §6.4): the admin sync's
    /// completion signal, correlated by the event's identity.
    events: broadcast::Sender<StravaEvent>,
    /// The in-memory wake signal (spec §4.4/§4.6): fired by `Registry::wake`
    /// after a Strava event is queued; the running loop selects on it, so
    /// the event is processed at once, even mid-idle-sleep.
    wake: Arc<Notify>,
    /// Aborts the executor task (`stop_all`): the watchdog records `Dead`
    /// on the abort `JoinError`, like a panic.
    task: tokio::task::AbortHandle,
}

/// The per-user executor registry (spec §4.4): the live executor per
/// [`UserId`], shared across requests (the `AppState` is `Clone`, so an
/// `Arc`). Spawn on the first demand (SSE connect or write), reap on idle
/// (the loop decides), respawn on the next demand — the streams re-register
/// on the respawned executor's frame channel.
#[derive(Clone)]
pub struct Registry {
    executors: Arc<Mutex<HashMap<UserId, Executor>>>,
    idle_timeout: Duration,
    heartbeat_interval: Duration,
}

impl Registry {
    pub fn new(idle_timeout: Duration, heartbeat_interval: Duration) -> Self {
        Self {
            executors: Arc::new(Mutex::new(HashMap::new())),
            idle_timeout,
            heartbeat_interval,
        }
    }

    /// The heartbeat cadence the SSE `KeepAlive` uses (the stream stays alive
    /// on `: ping` comments while the executor is reaped).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Register a stream for `user_id` (spawning the executor if it is not
    /// running) and return a subscription to its frame channel plus a handle
    /// to its fate, so the stream can tell a clean reap from a dead executor.
    pub async fn subscribe<S>(
        &self,
        source: &S,
        user_id: UserId,
    ) -> TbResult<(
        broadcast::Receiver<Summary>,
        watch::Receiver<ExecutorStatus>,
    )>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        self.exec_field(source, user_id, |exec| {
            (exec.frames.subscribe(), exec.status.clone())
        })
        .await
    }

    /// Fire the wake signal for the user's executor (spec §4.4/§4.6): a
    /// pure signal to a **running** executor — it interrupts the loop's
    /// idle wait so a just-queued Strava event is processed at once. It
    /// never spawns an executor: a reaped (or not-yet-spawned) executor
    /// picks the event up on its next spawn (an SSE connect or an API
    /// write) — the DB queue is the source of truth. Infallible by design:
    /// a missed wake only delays the event to the next spawn, so the
    /// webhook treats it as non-critical.
    pub async fn wake(&self, user_id: UserId) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            exec.wake.notify_one();
        }
    }
    /// Enqueue one [`ApiWrite`] on the user's executor and await its outcome
    /// (spec §6.2): spawn-or-join the executor, send the request on the
    /// user's API-write channel, and await the oneshot (unbounded — through
    /// a 15-minute rate-limit backoff the write may wait long, and that is
    /// the spec'd behavior). `exec_field` drops the registry lock on return,
    /// so the unbounded reply await runs without it.
    pub async fn write<S>(&self, source: &S, user_id: UserId, write: ApiWrite) -> TbResult<Summary>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        match self.write_outcome(source, user_id, write).await? {
            WriteOutcome::Summary(summary) => Ok(summary),
            WriteOutcome::Descend { summary, .. } => Ok(summary),
        }
    }

    /// Enqueue the Garmin CSV descend on the user's executor and await its
    /// match report (the spec §6.2 deviation recorded on issue #446): the
    /// report is the response body (`200 + {good, bad}`); the matched
    /// activities' state rides the stream frame like any other write. The
    /// spawn-or-join, enqueue, and unbounded await are `write`'s; only the
    /// outcome the route receives differs.
    pub async fn write_descend<S>(
        &self,
        source: &S,
        user_id: UserId,
        data: String,
    ) -> TbResult<DescendReport>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        match self
            .write_outcome(source, user_id, ApiWrite::ActivityDescend { data })
            .await?
        {
            WriteOutcome::Descend { report, .. } => Ok(report),
            // The descend write always carries its report; anything else is
            // a bug, not a domain outcome.
            WriteOutcome::Summary(_) => Err(Error::AnyFailure(anyhow::anyhow!(
                "the descend write did not carry its match report"
            ))),
        }
    }

    /// Enqueue one [`ApiWrite`] on the user's executor and await the full
    /// outcome (the `Summary` plus, for the descend, its match report):
    /// spawn-or-join the executor, send the request on the user's
    /// API-write channel, and await the oneshot (unbounded — through a
    /// 15-minute rate-limit backoff the write may wait long, and that is
    /// the spec'd behavior). `exec_field` drops the registry lock on
    /// return, so the unbounded reply await runs without it.
    async fn write_outcome<S>(
        &self,
        source: &S,
        user_id: UserId,
        write: ApiWrite,
    ) -> TbResult<WriteOutcome>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        let (request, reply) = ApiWriteRequest::new(write);
        let writer = self
            .exec_field(source, user_id, |exec| exec.writer.clone())
            .await?;
        if writer.send(request).is_err() {
            // The executor died between the liveness check and the send
            // (spec §4.6): the write is lost, the client retries.
            return Err(Error::AnyFailure(anyhow::anyhow!(
                "the executor for user {user_id} is gone"
            )));
        }
        reply.await.map_err(|_| {
            Error::AnyFailure(anyhow::anyhow!(
                "the executor for user {user_id} dropped the write"
            ))
        })?
    }

    /// Stop every live executor (the admin rescan's global re-hydration,
    /// issue #446): for each executor, abort the task (the watchdog records
    /// `Dead` on the abort `JoinError`, like a panic) — the aborted task
    /// drops its clones of the frame and consumed-event senders, and
    /// clearing the map drops the registry's, so both channels close and
    /// open streams see the close and end (their clients reconnect and the
    /// catch-up snapshot re-hydrates them; an in-flight admin sync hits its
    /// already-handled queue-fallback path) — then clear the map. The next
    /// `subscribe`/`write` respawns fresh (`ensure_running`'s existing
    /// behavior). The lock is held across the loop, so no executor can be
    /// spawned between the aborts and the clear.
    pub async fn stop_all(&self) {
        let mut map = self.executors.lock().await;
        for exec in map.values() {
            exec.task.abort();
        }
        map.clear();
    }

    /// A subscription to the user's executor's consumed-Strava-event channel
    /// (spec §6.4): spawning the executor if it is not running. The admin
    /// sync subscribes **before** it enqueues its event, so the completion
    /// signal cannot be missed.
    pub async fn events<S>(
        &self,
        source: &S,
        user_id: UserId,
    ) -> TbResult<broadcast::Receiver<StravaEvent>>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        self.exec_field(source, user_id, |exec| exec.events.subscribe())
            .await
    }

    /// Run one admin sync for `user_id` (spec §6.4): subscribe to the
    /// executor's consumed-event channel (spawning it if it is not running),
    /// enqueue a `Sync` event on the `strava_events` queue, wake the
    /// executor, and await that event's completion (unbounded — the spec'd
    /// behavior for the admin endpoint).
    ///
    /// The completion is correlated by the event's identity (owner, type,
    /// `event_time`). If the channel closes or lags (the executor settled
    /// mid-wait), the `strava_events` queue — the source of truth (spec
    /// §4.6) — is consulted: a sync no longer queued has been consumed.
    pub async fn sync<S>(
        &self,
        source: &S,
        user_id: UserId,
        strava_id: StravaId,
        event_time: i64,
    ) -> TbResult<()>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        // Subscribe before enqueuing, so the completion cannot be missed.
        let mut done = self.events(source, user_id).await?;

        // Enqueue the sync on the DB queue, like the webhook does.
        let mut store = source.begin().await?;
        tb_strava::event::insert_sync(strava_id, event_time, false, &mut store).await?;
        store.commit().await?;

        // Wake the executor (non-critical: the queue is the source of
        // truth). The `events` subscription above already ensured the
        // executor is running, so this only interrupts its idle wait.
        self.wake(user_id).await;

        // Await our sync's completion (unbounded, spec §6.4).
        loop {
            match done.recv().await {
                Ok(event) if is_sync(&event, strava_id, event_time) => return Ok(()),
                // Some other message completed; keep waiting for ours.
                Ok(_) => continue,
                Err(_) => {
                    // The channel closed (or lagged): the executor settled
                    // without announcing our event. Consult the queue — the
                    // source of truth (spec §4.6): a sync no longer queued
                    // has been consumed; otherwise re-subscribe (respawning
                    // the executor) and keep waiting.
                    if !self.sync_queued(source, strava_id, event_time).await? {
                        return Ok(());
                    }
                    done = self.events(source, user_id).await?;
                }
            }
        }
    }

    /// Whether `strava_id`'s queue still holds the sync at `event_time`
    /// (the admin sync's fallback completion check, spec §4.6).
    async fn sync_queued<S>(
        &self,
        source: &S,
        strava_id: StravaId,
        event_time: i64,
    ) -> TbResult<bool>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        let mut store = source.begin().await?;
        // A non-migrate sync is queued with `object_id` 0, so the user's
        // `object_id` 0 events are exactly their syncs.
        let syncs = store.strava_event_get_later(0, strava_id).await?;
        store.commit().await?;
        Ok(syncs.iter().any(|event| event.event_time == event_time))
    }

    /// Push a frame to the user's SSE streams (test helper: the production
    /// path pushes frames from the loop via `tb_exec::push_frame`).
    pub async fn send_frame(&self, user_id: UserId, summary: Summary) {
        let map = self.executors.lock().await;
        if let Some(exec) = map.get(&user_id) {
            let _ = exec.frames.send(summary);
        }
    }

    /// The user's executor's current fate (test helper).
    pub async fn status(&self, user_id: UserId) -> Option<ExecutorStatus> {
        let map = self.executors.lock().await;
        map.get(&user_id).map(|exec| *exec.status.borrow())
    }

    /// Lock the registry, ensure the user's executor is live (spawning it if
    /// it is absent or settled), and pull one field from it: the shared shape
    /// of `subscribe`, `wake`, `events`, and `write`. The lock is dropped
    /// when this returns, before the caller's next await.
    async fn exec_field<S, F, R>(&self, source: &S, user_id: UserId, f: F) -> TbResult<R>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
        F: FnOnce(&Executor) -> R,
    {
        let mut map = self.executors.lock().await;
        let exec = Self::ensure_running(&mut map, source, user_id, self.idle_timeout).await?;
        Ok(f(&exec))
    }

    /// Return the user's live executor, spawning it if it is absent or has
    /// settled (reaped/dead): read the stored
    /// [`StravaUser`](tb_strava::StravaUser) to build the executor's
    /// `StravaSession` (spec §4.4), open a fresh API-write channel and frame
    /// channel, and spawn the `tb_exec` loop.
    async fn ensure_running<S>(
        map: &mut HashMap<UserId, Executor>,
        source: &S,
        user_id: UserId,
        idle_timeout: Duration,
    ) -> TbResult<Executor>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            return Ok(exec.clone());
        }

        // The session read runs under the registry lock on purpose (the
        // single-node trade-off is deliberate): it is one row
        // (`stravauser_get_by_tbid`), and moving it out of the lock would pay
        // a database read on *every* demand — even with a live executor — to
        // save a sub-millisecond stall on other users' spawn paths.
        let mut store = source.begin().await?;
        let session = RequestSession::for_user(user_id, &mut store).await?;
        store.commit().await?;

        let (writer, reader) = api_write_channel();
        let (frames, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (events, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (status_tx, status_rx) = watch::channel(ExecutorStatus::Running);
        let wake = Arc::new(Notify::new());
        info!("spawning the executor for user {user_id}");

        // The task keeps a clone of the frame sender alive until *after* it
        // records its fate, so a stream that sees the frame channel close has
        // already seen the settled status (no race between the two signals).
        // `source`, `frames_for_run`, and `events_for_run` are owned clones:
        // the `async move` block needs `'static` values, not the `&S`
        // reference.
        let owned_source = source.clone();
        let frames_for_run = frames.clone();
        let events_for_run = events.clone();
        let wake_for_run = wake.clone();
        let watchdog_status = status_tx.clone();
        let join = tokio::spawn(async move {
            let result = run(
                owned_source,
                session,
                reader,
                frames_for_run,
                events_for_run,
                idle_timeout,
                wake_for_run,
            )
            .await;
            let _ = status_tx.send(match result {
                Ok(()) => ExecutorStatus::Reaped,
                Err(_) => ExecutorStatus::Dead,
            });
            result
        });
        // A panicked task skips the `status_tx.send` above; the watchdog
        // records the panic as `Dead` (like any other failure) so
        // `ensure_running` respawns instead of handing out a dead executor
        // forever (spec §4.4: the next SSE connect or API write respawns
        // it). The watchdog lives exactly as long as the executor task.
        //
        // The abort handle is taken before the watchdog moves `join`: it is
        // how `stop_all` tears the task down (rescan, issue #446) — the
        // watchdog's `join.await` then sees the abort `JoinError` and
        // records `Dead`, exactly like a panic.
        let task = join.abort_handle();
        tokio::spawn(async move {
            if join.await.is_err() {
                let _ = watchdog_status.send(ExecutorStatus::Dead);
            }
        });

        let exec = Executor {
            status: status_rx,
            writer,
            frames,
            events,
            wake,
            task,
        };
        map.insert(user_id, exec.clone());
        Ok(exec)
    }
}

/// The admin sync's completion correlation (spec §6.4): the consumed event
/// is ours when its identity matches the sync we queued.
fn is_sync(event: &StravaEvent, strava_id: StravaId, event_time: i64) -> bool {
    event.object_type == ObjectType::Sync
        && event.owner_id == strava_id
        && event.event_time == event_time
}

/// The seed for one SSE connection's `unfold`: the current frame subscription
/// and its executor's fate channel. A `None` frame subscription means "the
/// executor settled; re-subscribe (respawning it) on the next step".
struct StreamSeed<S: TxnSource + Clone + 'static> {
    app: AppState<S>,
    user_id: UserId,
    rx: Option<broadcast::Receiver<Summary>>,
    status: Option<watch::Receiver<ExecutorStatus>>,
}

/// One step of the SSE stream: ensure a live subscription (re-subscribing —
/// and respawning the executor — after it settles), then forward the next
/// frame. A frame is the `Summary` JSON (no envelope, no sequence). A clean
/// reap yields a heartbeat and re-subscribes on the next step; a dead executor
/// ends the stream (the client reconnects and respawns, spec §4.5).
async fn next_event<S>(seed: &mut StreamSeed<S>) -> Option<Result<Event, Infallible>>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let (mut rx, mut status) = match (seed.rx.take(), seed.status.take()) {
        (Some(rx), Some(status)) => (rx, status),
        _ => match seed
            .app
            .registry
            .subscribe(&seed.app.source, seed.user_id)
            .await
        {
            Ok(v) => v,
            Err(err) => {
                error!(
                    "the executor for user {} is gone and could not be respawned: {err:?}",
                    seed.user_id
                );
                return None;
            }
        },
    };

    tokio::select! {
        frame = rx.recv() => {
            match frame {
                Ok(summary) => {
                    seed.rx = Some(rx);
                    seed.status = Some(status);
                    Some(Ok(frame_event(&summary)))
                }
                // The frame channel closed: the executor settled. The status is
                // already set (the task wrote it before dropping its sender).
                Err(_) => settled_event(*status.borrow()),
            }
        }
        _ = status.changed() => {
            // The fate changed while we were waiting on a frame.
            let settled = status.borrow_and_update();
            seed.rx = None;
            seed.status = None;
            settled_event(*settled)
        }
    }
}

/// Map a settled executor fate to the stream's next output: a clean reap keeps
/// the connection alive (a heartbeat, then re-subscribe); a dead executor ends
/// the stream so the client reconnects.
fn settled_event(status: ExecutorStatus) -> Option<Result<Event, Infallible>> {
    match status {
        ExecutorStatus::Running => {
            // The channel closed before its fate was recorded (the narrow
            // window between the task dropping its frame sender and the
            // watchdog writing `Dead`): treat as dead — close the stream.
            None
        }
        ExecutorStatus::Reaped => Some(Ok(heartbeat_event())),
        ExecutorStatus::Dead => None,
    }
}

/// One SSE frame: the completed message's [`Summary`] as raw JSON (spec
/// §4.5 — no envelope, no type discriminator, no sequence number).
fn frame_event(summary: &Summary) -> Event {
    let data = serde_json::to_string(summary).expect("Summary always serializes");
    Event::default().data(data)
}

/// A heartbeat comment (`: ping`), invisible to the client's `onmessage`.
fn heartbeat_event() -> Event {
    Event::default().comment("ping")
}

/// `GET /api/user/stream` (spec §6.1): session-scoped, next to
/// `/api/user/summary` (the session extractor rejects a missing session with
/// 401). Spawn/get the user's executor, register this connection's stream,
/// and forward `Summary` frames (heartbeats while the executor is reaped).
/// Teardown (deregister; the executor reaps on idle once this was the last
/// stream) happens when the stream is dropped — the broadcast receiver's
/// count is the loop's stream-count signal.
pub(crate) async fn endpoint<S>(user: RequestSession, State(state): State<AppState<S>>) -> Response
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let user_id = user.user_id();
    let heartbeat = state.registry.heartbeat_interval();
    let (rx, status) = match state.registry.subscribe(&state.source, user_id).await {
        Ok(v) => v,
        Err(err) => {
            error!("spawning the executor for user {user_id} failed: {err:?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Executor spawn failed").into_response();
        }
    };

    let stream = unfold(
        StreamSeed {
            app: state,
            user_id,
            rx: Some(rx),
            status: Some(status),
        },
        |mut seed| async move { next_event(&mut seed).await.map(|item| (item, seed)) },
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(heartbeat).text("ping"))
        .into_response()
}
