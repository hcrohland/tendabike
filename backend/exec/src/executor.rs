/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

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
use std::time::Duration;

use log::{debug, error, info, warn};
use tb_domain::{Error, Summary, TbResult, WriteOutcome, exec};
use tb_strava::event::{Event, QueueRead, get_event, process};
use tb_strava::{StravaSession, StravaStore};
use tokio::sync::broadcast;

use crate::message::{ApiWriteReceiver, ApiWriteRequest, push_frame};
use crate::{Txn, TxnSource};

/// The per-executor in-memory backoff for database failures (spec #446
/// §4.6): exponential, capped at 30s, no `Stop` row (that is the Strava
/// rate limit's, global in the DB). A broken database must not hot-spin the
/// loop, and a recovering one is probed at human pace. Pure, so the state
/// machine is unit-testable without a store.
pub(crate) struct DbBackoff {
    next: Duration,
}

impl DbBackoff {
    /// The first pause after a DB failure.
    const BASE: Duration = Duration::from_secs(1);
    /// The longest pause: a database down for more than 30s at a time is
    /// probed every 30s, not less (spec #446 §4.6: "capped (e.g. 30s)").
    const CAP: Duration = Duration::from_secs(30);

    pub(crate) fn new() -> Self {
        Self { next: Self::BASE }
    }

    /// Records a DB failure and returns the pause to sleep: 1s, 2s, 4s, …,
    /// capped at 30s.
    pub(crate) fn record_failure(&mut self) -> Duration {
        let pause = self.next;
        self.next = (self.next * 2).min(Self::CAP);
        pause
    }

    /// A healthy database (any committed transaction, including a successful
    /// probe) restarts the backoff at its base.
    pub(crate) fn reset(&mut self) {
        self.next = Self::BASE;
    }
}

/// The sleep duration for a global rate-limit `Stop` deadline (unix seconds)
/// measured against `now` (unix seconds): saturating at zero, so an expired
/// deadline sleeps nothing and the next probe deletes the Stop and reads on.
/// Pure, so the deadline arithmetic is unit-testable without a clock.
pub(crate) fn stop_remaining(until_unix: i64, now_unix: i64) -> Duration {
    let secs = (until_unix as i128 - now_unix as i128).max(0);
    Duration::from_secs(secs as u64)
}

/// The current unix time in seconds (the clock `Stop` deadlines are written
/// in — `tb_strava`'s `get_time`): saturating at zero, since the deadline
/// arithmetic is what matters, not a pre-1970 clock.
pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// One decision of the loop's idle policy: what to do with a cycle in which
/// the biased select found work (or none). Pure so the reclaim rule is
/// unit-testable without a store (spec §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Process a pending API write first (a client action never waits).
    ProcessWrite,
    /// No write, but a Strava event is queued.
    ProcessStrava,
    /// No work; stay alive (streams still attached).
    Sleep,
    /// Reap: exit cleanly with `Ok(())` (spec §4.4 — the next write or SSE
    /// connect respawns the loop; an attached stream keeps the loop alive,
    /// so a reap always means no stream is attached).
    Reap,
}

/// The reclaim decision (spec §4.4, amended — an attached stream keeps the
/// executor alive), isolated from the async machinery:
///
/// - a pending API write always wins (the bias of the select);
/// - else a queued Strava event;
/// - else (nothing pending) the loop is reaped **only when there is no
///   active stream**. An attached stream keeps the loop alive: the idle
///   window (`idle_timeout`) is the cadence at which the idle loop
///   re-probes the `strava_events` queue, not a reap trigger — the stream's
///   TCP death is what ends it (its heartbeat write fails, the receiver
///   drops, and the no-streams rule reaps the loop).
pub fn next_action(pending_write: bool, queued_event: bool, active_streams: usize) -> Action {
    if pending_write {
        return Action::ProcessWrite;
    }
    if queued_event {
        return Action::ProcessStrava;
    }
    if active_streams == 0 {
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
    /// The global rate-limit `Stop` is active until this unix time: the loop
    /// sleeps until the deadline as a branch of the biased select (spec
    /// #446 §4.6) — API writes are never delayed by the backoff.
    Limited(i64),
    /// The queue holds nothing interesting for this user.
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
    /// The global rate-limit `Stop` is active until this unix time.
    Limited(i64),
    /// Both idle: no pending write and no queued event.
    Idle,
    /// The wake signal fired: a signal only (spec §4.4/§4.6) — the loop
    /// falls through to the next cycle, which re-probes the queue. It never
    /// goes through the reclaim policy (a fired wake must not reap an
    /// executor that has a just-queued event to process).
    Wake,
    /// The Strava probe failed (the queue's state is unknown).
    StravaError,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiWrite(req) => f.debug_tuple("Select::ApiWrite").field(req).finish(),
            Self::Strava(event) => f.debug_tuple("Select::Strava").field(event).finish(),
            Self::Limited(until) => f.debug_tuple("Select::Limited").field(until).finish(),
            Self::Idle => f.write_str("Select::Idle"),
            Self::Wake => f.write_str("Select::Wake"),
            Self::StravaError => f.write_str("Select::StravaError"),
        }
    }
}

/// One iteration of the biased select (spec §4.3): the API-write branch is
/// polled **first** (a client action must not wait behind a batch sync or a
/// backoff), then the Strava queue probe, then the wake signal; neither
/// resolving with work is the idle state the reclaim policy
/// ([`next_action`]) decides on.
///
/// `None` from the channel (its senders all dropped) resolves the write
/// branch without a message: production senders live for the process, so
/// this is a shutdown corner — the loop treats it as idle and the next
/// decision reaps it.
///
/// The wake branch (spec §4.4/§4.6) is a signal only: a fired wake resolves
/// the select to [`Select::Wake`], and the loop falls through to the next
/// cycle, which re-probes the queue. It is a `&mut` so the caller's
/// `notified()` future — created before the select, so a notify that fired
/// just before is not lost (the permit semantics) — survives the select and
/// can serve the loop's next parking spot (the idle sleep).
pub(crate) async fn select_message(
    api: impl Future<Output = Option<ApiWriteRequest>>,
    strava: impl Future<Output = Option<Probe>>,
    wake: &mut (impl Future<Output = ()> + Unpin),
) -> Select {
    tokio::select! {
        biased;
        req = api => match req {
            Some(req) => Select::ApiWrite(req),
            None => Select::Idle,
        },
        probe = strava => match probe {
            Some(Probe::Event(event)) => Select::Strava(event),
            Some(Probe::Limited(until)) => Select::Limited(until),
            Some(Probe::Empty) => Select::Idle,
            Some(Probe::Error) => Select::StravaError,
            None => Select::Idle,
        },
        // A fired wake is a signal only (spec §4.4/§4.6): the loop falls
        // through to the next cycle, which re-probes the queue. A ready
        // probe result is taken first — it is the queue's committed state;
        // the wake only says "check again".
        _ = wake => Select::Wake,
    }
}

/// The per-user executor loop (spec §4.3–4.5). One task per user, spawned
/// on demand (the first SSE connect or API write — web layer); it exits
/// `Ok(())` when reaped (no stream is attached) and is respawned on the
/// next demand. An attached stream keeps it alive: the idle window is the
/// queue re-probe cadence (spec §4.4, amended — #446).
///
/// One transaction per message (spec §4.3): `begin` → dispatch (`exec` for
/// an API write, `process` for a Strava event) → `commit` on success,
/// `rollback` on any failure (this is the caller the `Txn` seam exists for).
/// After a successful commit, a non-empty `Summary` pushes one frame to the
/// user's SSE streams ([`push_frame`]).
///
/// API writes fail fast: the oneshot resolves with the error and the message
/// is consumed (spec §6.2 — the client is the retry layer); the loop never
/// ends for a message failure. Strava-side failures are classified inside
/// `process` (spec §4.6 — the `Stop` for `TryAgain`, the disable for
/// `NotAuth`, the delete for a permanent failure); DB failures — a failed
/// `begin`/`commit`, a failed probe — pause the loop on the in-memory
/// [`DbBackoff`] (exponential, capped at 30s, biased so API writes are never
/// delayed) and retry after the pause. Only a failed `rollback` ends the
/// loop (its # Failure semantics section spells out each lane).
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
/// * `events` — the user's completed-Strava-event channel (`broadcast`):
///   every event the loop consumed (its processing transaction committed)
///   is sent on it, even with an empty summary. The web layer's admin sync
///   awaits its own queued event on this channel (spec §6.4) — the event's
///   identity lets the waiter correlate its sync.
/// * `idle_timeout` — the queue re-probe cadence while the loop is idle
///   with streams attached (spec §4.4, amended — #446): the loop is reaped
///   only when no stream is attached, so an attached stream keeps it alive
///   and the window sets how often the idle loop re-probes the
///   `strava_events` queue. A stream-less loop reaps immediately after its
///   work; the window never reaps it later.
/// * `wake` — the in-memory wake signal (spec §4.4/§4.6): the web layer's
///   `Registry` fires it after a Strava event is queued, so the loop's idle
///   wait is interrupted and the queue re-probed at once. The loop selects
///   on a `notified()` future created before each select (the permit
///   semantics: a notify that fires just before the future is created is
///   not lost); the same future serves both parking spots (the top-level
///   select and the idle sleep), so a wake that fires while the probe is in
///   flight is not dropped. A fired wake never spawns the loop — it is a
///   signal to this running loop only; a reaped loop picks the event up on
///   its next spawn (the DB queue is the source of truth).
///
/// # Failure semantics (spec §4.6)
///
/// Message failures are recoverable in the loop; only a failed `rollback`
/// ends it:
///
/// - **A failed `ApiWrite`** — fail fast, no retry: the oneshot resolves with
///   the error (the route maps it to a 4xx/5xx), the message is consumed (the
///   client is the retry layer), and the loop continues. A `begin` or `commit`
///   failure is a **DB failure**: the oneshot 500s, the in-memory backoff
///   records it, the loop continues.
/// - **A failed Strava message** — classified inside `process` (spec §4.6):
///   `TryAgain` inserts the global `Stop` (the probe then reports it and the
///   loop sleeps until its deadline); `NotAuth` disables the user and drops
///   their events; a permanent failure deletes the event; all of those commit
///   and the loop continues. A DB failure surfaces as `Err` from `process`:
///   the transaction rolls back (the event stays queued), the backoff records
///   it, the loop continues.
/// - **The global `Stop`** — the probe reports its deadline
///   ([`Select::Limited`]); the loop sleeps until it as a branch of the
///   biased select, so API writes interrupt the backoff and are never
///   delayed by it.
/// - **DB failures** (`begin`/`commit` failing, a failed probe) — the
///   per-executor in-memory backoff ([`DbBackoff`]): 1s, 2s, 4s, …, capped at
///   30s, no `Stop` row; the slept branch is biased, so a write arriving
///   mid-backoff is taken immediately. Any healthy transaction (a committed
///   write/event, a successful probe) resets the backoff.
/// - **A failed `rollback`** (either lane) returns `Err` from `run`: the
///   transaction's fate is not ours to reason about, so the loop ends. The
///   web layer (#456) treats an `Err` return like a panic — close the
///   streams, let the client reconnect and respawn — and the queue read
///   resumes on demand. There is no supervisor and no panic-loop guard: a
///   deterministic poison event is a visible alarm (the persistent
///   reconnect/refresh cycle) that the log names via `Processing {event}`.
///
/// On a successful commit a non-empty `Summary` pushes one frame to
/// [`frames`]; an empty `Summary` pushes nothing.
pub async fn run<T, S>(
    source: T,
    session: S,
    mut rx: ApiWriteReceiver,
    frames: broadcast::Sender<Summary>,
    events: broadcast::Sender<Event>,
    idle_timeout: Duration,
    wake: std::sync::Arc<tokio::sync::Notify>,
) -> TbResult<()>
where
    T: TxnSource + Clone + Send + Sync + 'static,
    T::Conn: StravaStore,
    S: StravaSession + Send + 'static,
{
    let mut session = session;
    let user_id = session.tb_id();
    let mut backoff = DbBackoff::new();

    loop {
        // One wake permit per cycle, created before any select (the permit
        // semantics: a notify that fires before the future is created is
        // not lost — `Notify` stores it). The same future serves both
        // parking spots (the top-level select and the idle sleep), so a
        // wake that fires while the probe is in flight is not dropped by
        // the select. Pinned: `Notified` is `!Unpin`, and the select takes
        // it by `&mut`.
        let mut wake_fut = Box::pin(wake.notified());
        let select = select_message(
            rx.recv(),
            queue_probe(&source, &session, &mut backoff),
            &mut wake_fut,
        )
        .await;

        match select {
            Select::ApiWrite(request) => {
                if let Err(err) =
                    run_write(&source, &mut session, request, &frames, &mut backoff).await
                {
                    error!("user {user_id}: the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::Strava(event) => {
                if let Err(err) = run_event(
                    &source,
                    &mut session,
                    &event,
                    &frames,
                    &events,
                    &mut backoff,
                )
                .await
                {
                    error!("user {user_id}: the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::Limited(until) => {
                // The global rate-limit `Stop` is active (spec §4.6): sleep
                // until its deadline as a branch of the biased select — an
                // API write arriving mid-sleep is taken the moment it is
                // ready, so the backoff delays only Strava events. The
                // deadline is absolute: after an interrupting write the loop
                // re-probes and sleeps the *remaining* time, not a fresh one.
                let remaining = stop_remaining(until, now_unix());
                if let Some(request) = sleep_or_take_write(&mut rx, remaining, &mut wake_fut).await
                    && let Err(err) =
                        run_write(&source, &mut session, request, &frames, &mut backoff).await
                {
                    error!("user {user_id}: the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::Wake => {
                // The wake signal fired (spec §4.4/§4.6): a signal only —
                // the loop falls through to the next cycle, which re-probes
                // the queue. It never goes through the reclaim policy (a
                // fired wake must not reap an executor that has a
                // just-queued event to process).
            }

            Select::StravaError => {
                // The probe failed; the queue's state is unknown. A DB
                // failure (spec §4.6): the in-memory backoff pauses the
                // re-probe (exponential, capped at 30s, biased so a write
                // arriving mid-pause is taken immediately); the failed event
                // (if any) stays queued for the retry.
                let pause = backoff.record_failure();
                warn!("user {user_id}: the queue probe failed; backing off {pause:?}");
                if let Some(request) = sleep_or_take_write(&mut rx, pause, &mut wake_fut).await
                    && let Err(err) =
                        run_write(&source, &mut session, request, &frames, &mut backoff).await
                {
                    error!("user {user_id}: the executor loop is ending: {err:?}");
                    return Err(err);
                }
            }

            Select::Idle => {
                let streams = frames.receiver_count();
                match next_action(false, false, streams) {
                    Action::Reap => {
                        info!("closing the executor for user {user_id}");
                        debug!(
                            "the executor loop is idle and reaping (user {user_id}, {streams} stream(s))"
                        );
                        return Ok(());
                    }
                    Action::Sleep => {
                        // Streams are still attached: stay alive (spec §4.4,
                        // amended — #446). The sleep is the queue re-probe
                        // cadence (a constant `idle_timeout`, not the
                        // remainder of an idle window — there is no window
                        // to run out). The wait is a biased select so a
                        // write arriving mid-wait is taken the moment it is
                        // ready (the bias of the select, honored across the
                        // wait too); a closed channel or the cadence
                        // expiring falls through to the next cycle's
                        // decision, which re-probes the queue.
                        if let Some(request) =
                            sleep_or_take_write(&mut rx, idle_timeout, &mut wake_fut).await
                            && let Err(err) =
                                run_write(&source, &mut session, request, &frames, &mut backoff)
                                    .await
                        {
                            error!("user {user_id}: the executor loop is ending: {err:?}");
                            return Err(err);
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

/// Sleep for `remaining` as a branch of the biased select (spec §4.6): an
/// API write arriving mid-sleep is returned the moment it is ready (a
/// backoff delays Strava events, never API writes); a fired wake (spec
/// §4.4/§4.6) interrupts the sleep with `None`, so the loop falls through
/// to the top and re-probes the queue at once; the sleep expiring (or the
/// channel closing — `None`, the shutdown corner) resolves with `None`.
async fn sleep_or_take_write(
    rx: &mut ApiWriteReceiver,
    remaining: Duration,
    wake: &mut (impl Future<Output = ()> + Unpin),
) -> Option<ApiWriteRequest> {
    tokio::select! {
        biased;
        request = rx.recv() => request,
        // A fired wake interrupts the idle sleep: the loop re-probes the
        // queue on its next cycle, so a just-queued event is processed at
        // once, not at the sleep's expiry (spec §4.4/§4.6).
        _ = wake => None,
        _ = tokio::time::sleep(remaining) => None,
    }
}

/// One Strava probe (the loop's select branch, one transaction): read the
/// queue with [`get_event`] (the read-side dedup lives on the Strava side) and
/// commit its housekeeping — the expired `Stop` and the dropped older
/// duplicates. The event that comes back is **not** deleted by the probe; the
/// work transaction's [`process`] read deletes it as part of the processing.
///
/// The probe is a health check as well as a queue read (spec §4.6): a
/// successful commit resets the DB backoff; a `begin`/read/commit failure is
/// a DB failure and returns [`Probe::Error`], which the loop answers with the
/// in-memory backoff (the housekeeping rolled back, so the queue's state is
/// unknown — the re-probe after the pause settles it).
async fn queue_probe<T: TxnSource, S: StravaSession>(
    source: &T,
    session: &S,
    backoff: &mut DbBackoff,
) -> Option<Probe>
where
    T::Conn: StravaStore,
{
    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!(
                "user {}: the queue probe could not begin a transaction: {err:?}",
                session.tb_id()
            );
            return Some(Probe::Error);
        }
    };
    let probe = match get_event(session, &mut conn).await {
        Ok(QueueRead::Event(event)) => Probe::Event(event),
        Ok(QueueRead::Limited { until }) => Probe::Limited(until),
        Ok(QueueRead::Empty) => Probe::Empty,
        Err(err) => {
            warn!("user {}: the queue probe failed: {err:?}", session.tb_id());
            let _ = conn.rollback().await;
            return Some(Probe::Error);
        }
    };
    if let Err(err) = conn.commit().await {
        // A failed commit is a DB failure (spec #446 §4.6): the housekeeping
        // rolled back, so report the queue's state as unknown — the loop
        // backs off and re-probes.
        warn!(
            "user {}: the queue probe could not commit its housekeeping: {err:?}",
            session.tb_id()
        );
        return Some(Probe::Error);
    }
    backoff.reset();
    debug!("queue probe: {probe:?}");
    Some(probe)
}

/// One `ApiWrite` in one transaction (spec §4.3): `begin` → [`exec`] →
/// `commit`, or `rollback` on failure; the oneshot resolves with the
/// outcome either way, and a successful, non-empty `Summary` pushes one
/// frame to the user's SSE streams.
///
/// Fail fast (spec §6.2, §4.6): the oneshot always resolves, the message is
/// consumed, and the loop continues — the client is the retry layer. A
/// `begin` or `commit` failure is a **DB failure**: the oneshot 500s, the
/// backoff records it, the loop continues (it does **not** end). Only a
/// failed `rollback` ends the loop with `Err` (the transaction's fate is not
/// ours to reason about); any other continued path proves the database
/// healthy and resets the backoff.
async fn run_write<T, S>(
    source: &T,
    session: &mut S,
    request: ApiWriteRequest,
    frames: &broadcast::Sender<Summary>,
    backoff: &mut DbBackoff,
) -> Result<(), Error>
where
    T: TxnSource,
    T::Conn: StravaStore,
    S: StravaSession,
{
    let ApiWriteRequest { write, reply } = request;
    info!("user {}: Processing {write:?}", session.tb_id());

    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            // DB failure (spec §4.6): 500 the client, back off, continue —
            // the write is consumed (the client retries), the loop stays up.
            error!("could not begin the write's transaction: {err:?}");
            let _ = reply.send(Err(Error::AnyFailure(err.into())));
            backoff.record_failure();
            return Ok(());
        }
    };

    let outcome = exec(write, session, &mut conn).await;
    let outcome: TbResult<WriteOutcome> = match outcome {
        Ok(outcome) => match conn.commit().await {
            Ok(()) => {
                backoff.reset();
                Ok(outcome)
            }
            Err(err) => {
                // DB failure (spec §4.6): 500 the client, back off, continue.
                // The write is consumed; if the commit in fact landed, the
                // client's retry meets the committed state and resolves
                // through the normal error mapping.
                error!("committing the write's transaction failed: {err:?}");
                backoff.record_failure();
                Err(Error::AnyFailure(err.into()))
            }
        },
        Err(err) => {
            // Fail fast (spec §6.2): the operation's partial writes roll
            // back, the oneshot carries the error, the message is consumed —
            // the client is the retry layer. The loop continues; the
            // database just ran a full transaction, so it is healthy.
            if let Err(rollback_err) = conn.rollback().await {
                error!("rolling back the write's transaction failed: {rollback_err:?}");
                let _ = reply.send(Err(Error::AnyFailure(rollback_err.into())));
                return Err(lifecycle_error(
                    "rolling back the write's transaction failed",
                ));
            }
            backoff.reset();
            Err(err)
        }
    };
    if let Ok(outcome) = &outcome {
        // The frame is the write's `Summary` (the descend's report is the
        // response body, not state — it never rides the frame).
        push_frame(frames, outcome.summary());
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
/// pushes one frame to the user's SSE streams, and the consumed event is
/// announced on the completion channel (spec §6.4 — fired even for an empty
/// summary, which pushes no frame).
///
/// [`process`] re-reads the queue with `get_event` inside the work
/// transaction — the accepted double read of the loop: the probe and the work
/// transaction are the same loop's two steps, and the queue is serialized per
/// user by that loop, so the re-read deterministically finds the same event
/// (the probe's committed housekeeping only removes rows `process` would
/// remove anyway).
///
/// `process` classifies the Strava-side failures itself (spec §4.6 — the
/// `Stop` for `TryAgain`, the disable for `NotAuth`, the delete for a
/// permanent failure), so the only `Err` it returns is a **DB failure**:
/// the transaction rolls back (the event stays queued), the backoff records
/// it, and the loop continues to retry after the pause. A `begin` or
/// `commit` failure is the same DB failure. Only a failed `rollback` ends
/// the loop with `Err` (the event is either still queued or already
/// processed — the respawn settles it on the next queue read).
async fn run_event<T, S>(
    source: &T,
    session: &mut S,
    event: &Event,
    frames: &broadcast::Sender<Summary>,
    events: &broadcast::Sender<Event>,
    backoff: &mut DbBackoff,
) -> Result<(), Error>
where
    T: TxnSource,
    T::Conn: StravaStore,
    S: StravaSession,
{
    info!("user {}: Processing {event}", session.tb_id());

    let mut conn = match source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            // DB failure (spec §4.6): back off, continue — the event stays
            // queued and is retried after the pause.
            warn!("could not begin a transaction for the event: {err:?}");
            backoff.record_failure();
            return Ok(());
        }
    };

    match process(session, &mut conn).await {
        Ok(summary) => match conn.commit().await {
            Ok(()) => {
                push_frame(frames, &summary);
                // The completion signal (spec §6.4): the event is consumed
                // for good, empty summary or not.
                let _ = events.send(event.clone());
                backoff.reset();
                Ok(())
            }
            Err(err) => {
                // DB failure (spec §4.6): back off, continue. The event is
                // either committed (the re-probe finds the next one) or still
                // queued (the re-probe finds it again) — either way the retry
                // after the pause settles it.
                error!("committing the event's transaction failed: {err:?}");
                backoff.record_failure();
                Ok(())
            }
        },
        Err(err) => {
            // The classifier left only DB failures (spec §4.6): roll back
            // (the event stays queued), record the failure, continue.
            warn!("the event's database failed and its transaction rolls back: {err:?}");
            if let Err(rollback_err) = conn.rollback().await {
                error!("the rollback failed as well: {rollback_err:?}");
                return Err(lifecycle_error(
                    "rolling back the event's transaction failed",
                ));
            }
            backoff.record_failure();
            Ok(())
        }
    }
}
