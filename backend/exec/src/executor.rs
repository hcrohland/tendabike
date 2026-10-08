//! The per-user executor loop (ADR-0005, executable spec #446 §4.3–4.5):
//! one long-lived task per user that consumes the two message sources
//! (the user's API-write channel, then the `strava_events` queue), runs each
//! message in one transaction, and pushes the resulting `Summary` to the
//! user's SSE streams.
//!
//! The loop is generic over the transaction source ([`TxnSource`]; the
//! production `DbPool` is injected at the composition root — `DbPool` is
//! `Clone` + `Send` + `Sync`, so no `Arc` wrapper is needed) and over the
//! user's [`StravaSession`]. It computes no domain state of its own: it
//! drives the existing `ApiWrite` dispatch and `tb_strava` event processing
//! inside transactions it owns.

use std::future::Future;
use std::time::{Duration, Instant};

use log::{error, info, warn};
use tb_domain::{Error, Summary, TbResult, exec};
use tb_strava::event::{Event, get_event, process};
use tb_strava::{StravaSession, StravaStore};
use tokio::sync::broadcast;

use crate::message::{ApiWriteReceiver, ApiWriteRequest, push_frame};
use crate::{Txn, TxnSource};

/// Fixed pause after a failed Strava message: a poison event (or a database
/// failure) must not hot-spin the loop, and the failed event stays queued
/// (its transaction rolled back) for the retry after the pause.
/// The full failure semantics of #455 — the rate-limit backoff state machine
/// (`Stop` rows) and the per-executor DB-failure backoff — replace this
/// fixed pause.
const STRAVA_ERROR_PAUSE: Duration = Duration::from_secs(1);

/// One decision of the loop's idle policy: what to do with a cycle in which
/// the biased select found work (or none). Pure so the reclaim rule is
/// unit-testable without a store (spec §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Process a pending API write first (a client action never waits).
    ProcessWrite,
    /// No write, but a Strava event is queued.
    ProcessStrava,
    /// No work; stay alive (streams still attached, idle window not over).
    Sleep,
    /// Reap: exit cleanly with `Ok(())` (spec §4.4 — the next write or SSE
    /// connect respawns the loop; attached streams keep heartbeating).
    Reap,
}

/// The reclaim decision (spec §4.4), isolated from the async machinery:
///
/// - a pending API write always wins (the bias of the select);
/// - else a queued Strava event;
/// - else (nothing pending) the loop is reaped when there is no active
///   stream, or when the idle window has elapsed with no messages — streams
///   that are still attached survive the reap on heartbeats (web layer) and
///   the next write or (re)connect respawns the loop.
pub fn next_action(
    pending_write: bool,
    queued_event: bool,
    active_streams: usize,
    idle: Duration,
    timeout: Duration,
) -> Action {
    if pending_write {
        return Action::ProcessWrite;
    }
    if queued_event {
        return Action::ProcessStrava;
    }
    if active_streams == 0 || idle >= timeout {
        Action::Reap
    } else {
        Action::Sleep
    }
}

/// The outcome of one read of the `strava_events` queue (one transaction).
/// Crate-internal: the loop and its unit tests are the only consumers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Probe {
    /// An event was queued; the work transaction will process it.
    Event(Event),
    /// The queue holds nothing interesting for this user (empty, or only an
    /// unexpired `Stop`).
    Empty,
    /// The probe failed (its transaction rolled back); the queue's state is
    /// unknown.
    Error,
}

/// One outcome of the biased select: which branch resolved, if any branch
/// resolved with work. Crate-internal.
pub(crate) enum Select {
    /// An API write arrived on the channel.
    ApiWrite(ApiWriteRequest),
    /// A Strava event was queued.
    Strava(Event),
    /// Both idle: no pending write and no queued event.
    Idle,
    /// The Strava probe failed (the queue's state is unknown).
    StravaError,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiWrite(req) => f.debug_tuple("Select::ApiWrite").field(req).finish(),
            Self::Strava(event) => f.debug_tuple("Select::Strava").field(event).finish(),
            Self::Idle => f.write_str("Select::Idle"),
            Self::StravaError => f.write_str("Select::StravaError"),
        }
    }
}

/// One iteration of the biased select (spec §4.3): the API-write branch is
/// polled **first** (a client action must not wait behind a batch sync or a
/// backoff), then the Strava queue probe; neither resolving with work is
/// the idle state the reclaim policy ([`next_action`]) decides on.
///
/// `None` from the channel (its senders all dropped) resolves the write
/// branch without a message: production senders live for the process, so
/// this is a shutdown corner — the loop treats it as idle and the next
/// decision reaps it.
pub(crate) async fn select_message(
    api: impl Future<Output = Option<ApiWriteRequest>>,
    strava: impl Future<Output = Option<Probe>>,
) -> Select {
    tokio::select! {
        biased;
        req = api => match req {
            Some(req) => Select::ApiWrite(req),
            None => Select::Idle,
        },
        probe = strava => match probe {
            Some(Probe::Event(event)) => Select::Strava(event),
            Some(Probe::Empty) => Select::Idle,
            Some(Probe::Error) => Select::StravaError,
            None => Select::Idle,
        },
    }
}

/// The per-user executor loop (spec §4.3–4.5). One task per user, spawned
/// on demand (the first SSE connect or API write — web layer); it exits
/// `Ok(())` when reaped on idle and is respawned on the next demand.
///
/// One transaction per message (spec §4.3): `begin` → dispatch (`exec` for
/// an API write, `process` for a Strava event) → `commit` on success,
/// `rollback` on any failure (this is the caller the `Txn` seam exists for).
/// After a successful commit, a non-empty `Summary` pushes one frame to the
/// user's SSE streams ([`push_frame`]).
///
/// API writes fail fast: the oneshot resolves with the error and the message
/// is consumed (spec §6.2 — the client is the retry layer). Failed Strava
/// messages log and pause [`STRAVA_ERROR_PAUSE`] so a poison event cannot
/// hot-spin the loop; the failed event stays queued and is retried after the
/// pause (the #455 backoff state machine replaces the fixed pause).
///
/// # Arguments
///
/// * `source` — the transaction source (`DbPool` in production). Taken by
///   value and owned for the loop's life; `Clone` (which `DbPool` is) lets
///   the caller keep a handle — no `Arc` is needed.
/// * `session` — the user's `StravaSession` (drives the queue read and
///   event processing; the executor supplies the identity `ApiWrite` needs).
/// * `rx` — the receive half of the user's API-write channel.
/// * `frames` — the user's SSE frame sink (`broadcast`); the web layer's
///   stream tasks subscribe to it.
/// * `idle_timeout` — how long the loop idles (no pending work) before it
///   reaps, while streams are still attached.
///
/// # Failure semantics
///
/// Message failures are recoverable in the loop (spec §4.6); transaction
/// lifecycle failures are not, and end it:
///
/// - **A failed `ApiWrite`** — a domain error (the operation ran inside the
///   transaction) or a `begin` failure: the oneshot resolves with the error
///   (the route maps it to a 4xx/5xx), the message is consumed (the client
///   is the retry layer), and the loop continues.
/// - **A failed Strava message** — a domain error: the transaction rolls
///   back, so the event stays queued; the loop logs and pauses
///   [`STRAVA_ERROR_PAUSE`] (the #455 backoff replaces the fixed pause), then
///   continues.
/// - **A transaction lifecycle failure** (`commit` or `rollback` erroring, on
///   either lane) returns `Err` from `run`: the transaction's fate is not
///   ours to reason about, so the loop ends. The web layer (#456) treats an
///   `Err` return like a panic — close the streams, let the client
///   reconnect and respawn — and the queue read resumes on demand.
///
/// On a successful commit a non-empty `Summary` pushes one frame to
/// [`frames`]; an empty `Summary` pushes nothing.
pub async fn run<T, S>(
    source: T,
    session: S,
    mut rx: ApiWriteReceiver,
    frames: broadcast::Sender<Summary>,
    idle_timeout: Duration,
) -> TbResult<()>
where
    T: TxnSource + Clone + Send + Sync + 'static,
    T::Conn: StravaStore,
    S: StravaSession + Send + 'static,
{
    let mut session = session;
    let mut last_activity = Instant::now();

    loop {
        let select = select_message(rx.recv(), queue_probe(&source, &session)).await;

        match select {
            Select::ApiWrite(request) => {
                last_activity = Instant::now();
                if let Err(err) = run_write(&source, &mut session, request, &frames).await {
                    error!("the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::Strava(event) => {
                last_activity = Instant::now();
                if let Err(err) = run_event(&source, &mut session, &event, &frames).await {
                    error!("the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::StravaError => {
                // The probe failed; the queue's state is unknown. Pause so a
                // broken database or a poison probe cannot hot-spin the loop;
                // the failed event (if any) stays queued for the retry.
                warn!("the queue probe failed; pausing {STRAVA_ERROR_PAUSE:?}");
                last_activity = Instant::now();
                tokio::time::sleep(STRAVA_ERROR_PAUSE).await;
            }

            Select::Idle => {
                let idle = last_activity.elapsed();
                let streams = frames.receiver_count();
                match next_action(false, false, streams, idle, idle_timeout) {
                    Action::Reap => {
                        info!(
                            "the executor loop is idle and reaping (idle {idle:?}, {streams} stream(s))"
                        );
                        return Ok(());
                    }
                    Action::Sleep => {
                        // Streams are still attached and the idle window is
                        // not over: stay alive. The wait is inside a select
                        // so a write arriving mid-wait is taken the moment it
                        // is ready (the bias of the select, honored across the
                        // wait too); a closed channel or the window expiring
                        // falls through to the next cycle's decision.
                        let remaining = idle_timeout - idle;
                        tokio::select! {
                            biased;
                            request = rx.recv() => {
                                if let Some(request) = request {
                                    last_activity = Instant::now();
                                    if let Err(err) =
                                        run_write(&source, &mut session, request, &frames)
                                            .await
                                    {
                                        error!("the executor loop is ending: {err:?}");
                                        return Err(err);
                                    }
                                }
                            }
                            _ = tokio::time::sleep(remaining) => {}
                        }
                    }
                    // `next_action` hands out `ProcessWrite`/`ProcessStrava`
                    // only for inputs this branch never has (it already
                    // checked both). `Sleep` and `Reap` are exhaustive here.
                    Action::ProcessWrite | Action::ProcessStrava => {}
                }
            }
        }
    }
}

/// One Strava probe (the loop's select branch, one transaction): read the
/// queue with [`get_event`] (the read-side dedup lives on the Strava side) and
/// commit its housekeeping — the expired `Stop` and the dropped older
/// duplicates. The event that comes back is **not** deleted by the probe; the
/// work transaction's [`process`] read deletes it as part of the processing.
///
/// The probe's commit is best-effort: a failed commit undoes only the
/// housekeeping, which `process`'s own `get_event` read redoes — the event
/// itself was never deleted, so the probe's verdict stands.
async fn queue_probe<T: TxnSource, S: StravaSession>(source: &T, session: &S) -> Option<Probe>
where
    T::Conn: StravaStore,
{
    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!("the queue probe could not begin a transaction: {err:?}");
            return Some(Probe::Error);
        }
    };
    let probe = match get_event(session, &mut conn).await {
        Ok(Some(event)) => Probe::Event(event),
        Ok(None) => Probe::Empty,
        Err(err) => {
            warn!("the queue probe failed: {err:?}");
            let _ = conn.rollback().await;
            return Some(Probe::Error);
        }
    };
    if let Err(err) = conn.commit().await {
        warn!("the queue probe could not commit its housekeeping: {err:?}");
    }
    Some(probe)
}

/// One `ApiWrite` in one transaction (spec §4.3): `begin` → [`exec`] →
/// `commit`, or `rollback` on failure; the oneshot resolves with the
/// outcome either way, and a successful, non-empty `Summary` pushes one
/// frame to the user's SSE streams.
///
/// Returns `Err` only on a transaction lifecycle failure (a failed
/// `commit` or `rollback`) — that ends the loop; a domain error of
/// [`exec`] is reported through the oneshot and the loop continues.
async fn run_write<T, S>(
    source: &T,
    session: &mut S,
    request: ApiWriteRequest,
    frames: &broadcast::Sender<Summary>,
) -> Result<(), Error>
where
    T: TxnSource,
    T::Conn: StravaStore,
    S: StravaSession,
{
    let ApiWriteRequest { write, reply } = request;
    info!("Processing {write:?}");

    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            error!("could not begin the write's transaction: {err:?}");
            let _ = reply.send(Err(Error::AnyFailure(err.into())));
            return Err(lifecycle_error("could not begin the write's transaction"));
        }
    };

    let summary = exec(write, session, &mut conn).await;
    let outcome: TbResult<Summary> = match summary {
        Ok(summary) => match conn.commit().await {
            Ok(()) => Ok(summary),
            Err(err) => {
                error!("committing the write's transaction failed: {err:?}");
                let _ = reply.send(Err(Error::AnyFailure(err.into())));
                return Err(lifecycle_error("committing the write's transaction failed"));
            }
        },
        Err(err) => {
            // Fail fast (spec §6.2): the operation's partial writes roll
            // back, the oneshot carries the error, the message is consumed —
            // the client is the retry layer. The loop continues.
            if let Err(rollback_err) = conn.rollback().await {
                error!("rolling back the write's transaction failed: {rollback_err:?}");
                let _ = reply.send(Err(Error::AnyFailure(rollback_err.into())));
                return Err(lifecycle_error(
                    "rolling back the write's transaction failed",
                ));
            }
            Err(err)
        }
    };
    if let Ok(summary) = &outcome {
        push_frame(frames, summary);
    }
    if let Err(err) = &outcome {
        error!("the API write failed: {err:?}");
    }
    let _ = reply.send(outcome);
    Ok(())
}

/// The `Err` value `run` returns when a message's transaction lifecycle
/// fails: a marker distinct from any domain error of the message itself. The
/// original error was already handed to the message's reply (a failed write
/// must 500 the client), so what ends the loop is a fresh marker. The web
/// layer treats an `Err` from `run` like a panic (close the streams, let the
/// client reconnect and respawn); the original is named in the log.
fn lifecycle_error(what: &str) -> Error {
    Error::AnyFailure(anyhow::anyhow!("the executor loop's transaction: {what}"))
}

/// One Strava event in one transaction (spec §4.3): `begin` → [`process`]
/// → `commit`, or `rollback` on failure; a successful, non-empty `Summary`
/// pushes one frame to the user's SSE streams.
///
/// [`process`] re-reads the queue with `get_event` inside the work
/// transaction — the accepted double read of the loop: the probe and the work
/// transaction are the same loop's two steps, and the queue is serialized per
/// user by that loop, so the re-read deterministically finds the same event
/// (the probe's committed housekeeping only removes rows `process` would
/// remove anyway).
///
/// A domain error keeps the event queued (the transaction rolled back) and
/// pauses the loop [`STRAVA_ERROR_PAUSE`] (the #455 backoff replaces the
/// fixed pause); a transaction lifecycle failure ends the loop with `Err`
/// (the event is either still queued or already processed — the respawn
/// settles it on the next queue read).
async fn run_event<T, S>(
    source: &T,
    session: &mut S,
    event: &Event,
    frames: &broadcast::Sender<Summary>,
) -> Result<(), Error>
where
    T: TxnSource,
    T::Conn: StravaStore,
    S: StravaSession,
{
    info!("Processing {event}");

    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            error!("could not begin a transaction for the event: {err:?}");
            return Err(err);
        }
    };

    match process(session, &mut conn).await {
        Ok(summary) => match conn.commit().await {
            Ok(()) => {
                push_frame(frames, &summary);
                Ok(())
            }
            Err(err) => {
                error!("the commit failed: {err:?}");
                Err(err)
            }
        },
        Err(err) => {
            warn!("the event failed and its transaction rolls back: {err:?}");
            if let Err(rollback_err) = conn.rollback().await {
                error!("the rollback failed as well: {rollback_err:?}");
                return Err(rollback_err);
            }
            tokio::time::sleep(STRAVA_ERROR_PAUSE).await;
            Ok(())
        }
    }
}
