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
use log::error;
use tokio::sync::{Mutex, broadcast, watch};

use tb_domain::{Session, Summary, TbResult, UserId};
use tb_exec::{ApiWriteSender, Txn, TxnSource, api_write_channel, run};
use tb_strava::StravaStore;

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
/// tasks). `Running` is the initial value; the task records `Reaped` or `Dead`
/// on exit (a panic leaves it `Running` — the frame channel has closed by
/// then, which is the disambiguating signal).
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
/// user's API-write channel (the write handlers enqueue here — next ticket),
/// and the SSE frame sink the stream tasks subscribe to.
struct Executor {
    status: watch::Receiver<ExecutorStatus>,
    /// The write handlers' door into this user's loop (spec §6.2). Held here
    /// so the next ticket's enqueue-and-await handlers can reach it; unused
    /// until they land.
    #[allow(dead_code)]
    writer: ApiWriteSender,
    frames: broadcast::Sender<Summary>,
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
    /// to its fate, so the stream can tell a clean reap from a panic.
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
        let mut map = self.executors.lock().await;
        let (frames, status) =
            Self::ensure_running(&mut map, source, user_id, self.idle_timeout).await?;
        Ok((frames.subscribe(), status))
    }

    /// Wake the executor for `user_id` (spawning it if it is not running) so a
    /// just-queued Strava event is picked up on its next wake (spec §4.6).
    /// Non-critical: the DB queue is the source of truth, so a failed wake
    /// must not fail the ingest.
    pub async fn wake<S>(&self, source: &S, user_id: UserId) -> TbResult<()>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        let mut map = self.executors.lock().await;
        let _ = Self::ensure_running(&mut map, source, user_id, self.idle_timeout).await?;
        Ok(())
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

    /// Return the user's live executor's frame sink and fate channel, spawning
    /// it if it is absent or has settled (reaped/dead): read the stored
    /// [`StravaUser`](tb_strava::StravaUser) to build the executor's
    /// `StravaSession` (spec §4.4), open a fresh API-write channel and frame
    /// channel, and spawn the `tb_exec` loop.
    async fn ensure_running<S>(
        map: &mut HashMap<UserId, Executor>,
        source: &S,
        user_id: UserId,
        idle_timeout: Duration,
    ) -> TbResult<(broadcast::Sender<Summary>, watch::Receiver<ExecutorStatus>)>
    where
        S: TxnSource + Clone + 'static,
        S::Conn: StravaStore,
    {
        if let Some(exec) = map.get(&user_id)
            && *exec.status.borrow() == ExecutorStatus::Running
        {
            return Ok((exec.frames.clone(), exec.status.clone()));
        }

        // Build the executor's `StravaSession` from the stored `StravaUser`
        // (the refresh token), the same read `create_from_id` does minus the
        // admin gate: an empty access token with a past expiry, so the first
        // Strava request forces a refresh. A read-only transaction.
        let mut store = source.begin().await?;
        let session = RequestSession::for_user(user_id, &mut store).await?;
        store.commit().await?;

        let (writer, reader) = api_write_channel();
        let (frames, _) = broadcast::channel(BROADCAST_CAPACITY);
        let (status_tx, status_rx) = watch::channel(ExecutorStatus::Running);

        // The task keeps a clone of the frame sender alive until *after* it
        // records its fate, so a stream that sees the frame channel close has
        // already seen the settled status (no race between the two signals).
        // `source` and `frames_for_run` are owned clones: the `async move`
        // block needs `'static` values, not the `&S` reference.
        let owned_source = source.clone();
        let frames_for_run = frames.clone();
        let join = tokio::spawn(async move {
            let result = run(owned_source, session, reader, frames_for_run, idle_timeout).await;
            let _ = status_tx.send(match result {
                Ok(()) => ExecutorStatus::Reaped,
                Err(_) => ExecutorStatus::Dead,
            });
            result
        });
        // The fate is observed through `status_rx`, not the handle: drop it
        // explicitly (a `JoinHandle` that is neither awaited nor dropped would
        // be a leaked task the registry never reaps).
        std::mem::drop(join);

        let exec = Executor {
            status: status_rx.clone(),
            writer,
            frames: frames.clone(),
        };
        map.insert(user_id, exec);
        Ok((frames, status_rx))
    }
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
            // The channel closed but the fate is not yet recorded (a panic,
            // which never records): treat as dead — close the stream.
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
