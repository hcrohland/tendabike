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

//! The per-user executor loop (ADR-0005, executable spec #446 §4.3–4.5;
//! ADR-0007, executable spec #558 §3): one long-lived task per user that
//! consumes the two message sources (the user's request channel — writes,
//! reads, the hydration write, the admin stats read — then the
//! `strava_events` queue), runs each message in one transaction, and pushes
//! the resulting `Summary` to the user's SSE streams.
//!
//! The loop is generic over the transaction source ([`TxnSource`]; the
//! production `DbPool` is injected at the composition root — `DbPool` is
//! `Clone` + `Send` + `Sync`, so no `Arc` wrapper is needed) and over the
//! user's [`Session`]. It computes no domain state of its own: it
//! drives the existing `ApiWrite` dispatch and `tb_strava` event processing
//! inside transactions it owns.

use std::future::Future;
use std::time::Duration;

use log::{debug, error, warn};
use tb_domain::{
    ApiRead, ApiWrite, Error, ReadOutcome, Session, ShopId, Summary, TbResult, UserId,
    WriteOutcome, exec,
};
use tb_strava::event::{Event, QueueRead, get_event, insert_sync, process};
use tb_strava::{StravaStat, StravaUser, get_all_stats};
use tokio::sync::{broadcast, oneshot};

use crate::message::{Request, RequestReceiver, push_frame};
use crate::{FramePusher, Txn, TxnSource};

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
    /// An API request (write, read, hydration, admin stats) arrived on the
    /// channel.
    Request(Request),
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
            Self::Request(req) => f.debug_tuple("Select::Request").field(req).finish(),
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
    api: impl Future<Output = Option<Request>>,
    strava: impl Future<Output = Option<Probe>>,
    wake: &mut (impl Future<Output = ()> + Unpin),
) -> Select {
    tokio::select! {
        biased;
        req = api => match req {
            Some(req) => Select::Request(req),
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

/// Route a completed `Summary` to its delivery target(s) by the session's
/// context (spec #517, ticket #584):
///
/// - **user context** (`shop == None`): the executor's own personal stream —
///   the existing [`push_frame`] behavior, unchanged.
/// - **shop context** (`shop == Some(shop_id)`): the shop stream
///   ([`FramePusher::push_to_shop`]) **and** the part owner's personal stream
///   ([`FramePusher::push_to_user`]) — never the executor's own stream (a
///   foreign part must not leak into the shop owner's personal view; the part
///   owner, who may be the shop owner, receives the frame via `push_to_user`).
///
/// **Cross-recipient notification routing** (issue #677): after the normal
/// frame is delivered, each notification in the `Summary` whose recipient is
/// not the acting user (and not the part owner in a shop context) is pushed
/// to that recipient's personal stream via [`FramePusher::push_to_user`]. A
/// recipient whose executor is not running gets nothing live — the row is in
/// the DB and reaches them on the next hydration.
///
/// An empty `Summary` pushes nothing, to any stream (spec §4.5).
pub(crate) async fn deliver_frame(
    pusher: &dyn FramePusher,
    frames: &broadcast::Sender<Summary>,
    user_id: UserId,
    shop: Option<ShopId>,
    part_owner: Option<UserId>,
    summary: &Summary,
) {
    if summary.is_empty() {
        return;
    }
    // Push the normal frame to the acting user's delivery target(s).
    match shop {
        None => push_frame(frames, summary),
        Some(shop_id) => {
            pusher.push_to_shop(shop_id, summary.clone()).await;
            if let Some(owner) = part_owner {
                pusher.push_to_user(owner, summary.clone()).await;
            }
        }
    }
    // Cross-recipient notification routing: push each notification to its
    // recipient's personal stream. The acting user's own notifications ride
    // the normal frame above; in a shop context the part owner's
    // notifications also ride the normal frame (pushed to their personal
    // stream). In a shop context the notification is also pushed to the shop
    // stream so a recipient whose client is on the shop stream (because they
    // are "in" the shop) receives it.
    for notification in summary.notifications.live() {
        let recipient = notification.recipient;
        if recipient == user_id {
            continue;
        }
        if let Some(owner) = part_owner
            && recipient == owner
        {
            continue;
        }
        let mut recipient_summary = Summary::default();
        recipient_summary.notifications.upsert(notification);
        pusher
            .push_to_user(recipient, recipient_summary.clone())
            .await;
        if let Some(shop_id) = shop {
            pusher.push_to_shop(shop_id, recipient_summary).await;
        }
    }
}

/// The per-user executor loop's context (issue #661): the loop's wiring and
/// its mutable state in one type, so the loop's full set of dependencies is
/// readable from one struct definition and its stateful behavior has a
/// single home.
///
/// The **wiring** the loop carries for its life:
///
/// - `source` — the transaction source (`DbPool` in production). Owned for
///   the loop's life; `Clone` (which `DbPool` is) lets the caller keep a
///   handle — no `Arc` is needed.
/// - `user_id` — the user the loop serves: the queue probe reads the queue
///   by it, and the event lane builds an identity-only [`ExecutorSession`]
///   from it (at the last moment) to drive the domain event processing.
/// - `frames` — the user's SSE frame sink (`broadcast`); the web layer's
///   stream tasks subscribe to it. In a **user context** (`shop() == None`)
///   a committed `Summary` is pushed here (the existing behavior).
/// - `pusher` — the frame push surface (the [`Registry`] in production) the
///   loop uses to route a committed `Summary` in a **shop context**
///   (`shop() == Some`): the shop stream plus the part owner's personal
///   stream (spec #517, ticket #584). In a user context the loop uses its
///   own `frames` channel and never the pusher.
/// - `events` — the user's completed-Strava-event channel (`broadcast`):
///   every event the loop consumed (its processing transaction committed)
///   is sent on it, even with an empty summary. The web layer's admin sync
///   awaits its own queued event on this channel (spec §6.4) — the event's
///   identity lets the waiter correlate its sync.
/// - `idle_timeout` — the queue re-probe cadence while the loop is idle
///   with streams attached (spec §4.4, amended — #446): the loop is reaped
///   only when no stream is attached, so an attached stream keeps it alive
///   and the window sets how often the idle loop re-probes the
///   `strava_events` queue. A stream-less loop reaps immediately after its
///   work; the window never reaps it later.
/// - `wake` — the in-memory wake signal (spec §4.4/§4.6): the web layer's
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
/// The **state** the loop mutates as it runs:
///
/// - `backoff` — the per-executor in-memory DB-failure backoff
///   ([`DbBackoff`]), started at its base.
/// - `first_idle` — the first-idle flag: the first idle cycle never reaps
///   (the executor just spawned, and the client's stream is likely to
///   connect moments later), started set.
///
/// The **request channel is not part of the context**: it is the loop's
/// input stream, conceptually distinct from the wiring the loop carries —
/// [`LoopContext::run`] takes it as its only argument.
///
/// Named for the loop, not the session: "context" is already loaded in this
/// codebase as the session's shop-scoping (the active shop value), and a
/// bare `Context` would also collide with the standard-library and
/// `anyhow` context types.
///
/// Public (with a public constructor) because the integration (seam) suite
/// — a separate crate — constructs it to drive the loop over a real
/// transaction source, the same single construction path as production.
pub struct LoopContext<T>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    /// The transaction source (`DbPool` in production).
    source: T,
    /// The user the loop serves.
    user_id: UserId,
    /// The user's SSE frame sink (`broadcast`).
    frames: broadcast::Sender<Summary>,
    /// The frame push surface (the [`Registry`] in production).
    pusher: Box<dyn FramePusher>,
    /// The user's completed-Strava-event channel (`broadcast`).
    events: broadcast::Sender<Event>,
    /// The queue re-probe cadence while the loop is idle with streams
    /// attached.
    idle_timeout: Duration,
    /// The in-memory wake signal (the [`Registry`] fires it after a Strava
    /// event is queued).
    wake: std::sync::Arc<tokio::sync::Notify>,
    /// The per-executor in-memory DB-failure backoff.
    backoff: DbBackoff,
    /// The first-idle flag: the first idle cycle never reaps.
    first_idle: bool,
}

impl<T> LoopContext<T>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    /// Assemble the loop's wiring, starting its mutable state (the backoff
    /// at its base, the first-idle flag set) — the two state locals the
    /// entry point used to declare now initialize here.
    pub fn new(
        source: T,
        user_id: UserId,
        frames: broadcast::Sender<Summary>,
        pusher: Box<dyn FramePusher>,
        events: broadcast::Sender<Event>,
        idle_timeout: Duration,
        wake: std::sync::Arc<tokio::sync::Notify>,
    ) -> Self {
        Self {
            source,
            user_id,
            frames,
            pusher,
            events,
            idle_timeout,
            wake,
            backoff: DbBackoff::new(),
            first_idle: true,
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
    /// * `rx` — the receive half of the user's request channel (writes, reads,
    ///   the hydration write, and the admin stats read — spec #558 §3.2). The
    ///   rest of the loop's wiring is the context's ([`LoopContext`]).
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
    /// - **A failed `ApiRead`** (spec #558 §3.7) — fail fast, no retry, and
    ///   **never arm the backoff**: the oneshot resolves with the error (the
    ///   route maps it to a 4xx/5xx), the message is consumed, and the loop
    ///   continues — the queue probe is the DB health monitor, and read traffic
    ///   (e.g. a reconnecting client's burst) must not push the 30s cap up. A
    ///   read transaction that completes cleanly (committed, or rolled back
    ///   after a domain error) resets the backoff, like any healthy
    ///   transaction. A read pushes no SSE frame (read-only within its
    ///   transaction by spec).
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
    pub async fn run(mut self, mut rx: RequestReceiver) -> TbResult<()> {
        loop {
            // One wake permit per cycle, created before any select (the permit
            // semantics: a notify that fires before the future is created is
            // not lost — `Notify` stores it). The same future serves both
            // parking spots (the top-level select and the idle sleep), so a
            // wake that fires while the probe is in flight is not dropped by
            // the select. Pinned: `Notified` is `!Unpin`, and the select takes
            // it by `&mut`. The `Arc` is cloned per cycle so the pinned future
            // borrows the clone, not the context (the queue probe borrows the
            // context mutably).
            let wake = self.wake.clone();
            let mut wake_fut = Box::pin(wake.notified());
            let select = select_message(rx.recv(), queue_probe(&mut self), &mut wake_fut).await;

            match select {
                Select::Request(request) => {
                    if let Err(err) = run_request(&mut self, request).await {
                        error!(
                            "user {}: the executor loop is ending: {err:?}",
                            self.user_id
                        );
                        return Err(err);
                    }
                }

                Select::Strava(event) => {
                    if let Err(err) = run_event(&mut self, &event).await {
                        error!(
                            "user {}: the executor loop is ending: {err:?}",
                            self.user_id
                        );
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
                    if let Some(request) =
                        sleep_or_take_request(&mut rx, remaining, &mut wake_fut).await
                        && let Err(err) = run_request(&mut self, request).await
                    {
                        error!(
                            "user {}: the executor loop is ending: {err:?}",
                            self.user_id
                        );
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
                    let pause = self.backoff.record_failure();
                    warn!(
                        "user {}: the queue probe failed; backing off {pause:?}",
                        self.user_id
                    );
                    if let Some(request) =
                        sleep_or_take_request(&mut rx, pause, &mut wake_fut).await
                        && let Err(err) = run_request(&mut self, request).await
                    {
                        error!(
                            "user {}: the executor loop is ending: {err:?}",
                            self.user_id
                        );
                        return Err(err);
                    }
                }

                Select::Idle => {
                    let streams = self.frames.receiver_count();
                    let action = next_action(false, false, streams);
                    // Don't reap on the first idle cycle: the executor just
                    // spawned (a read or write triggered it), and the client's
                    // stream is likely to connect moments later. The first idle
                    // cycle sleeps instead, giving the stream time to attach.
                    let action = if action == Action::Reap && self.first_idle {
                        self.first_idle = false;
                        Action::Sleep
                    } else {
                        action
                    };
                    match action {
                        Action::Reap => {
                            debug!(
                                "closing the executor for user {} (idle, {streams} stream(s))",
                                self.user_id
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
                                sleep_or_take_request(&mut rx, self.idle_timeout, &mut wake_fut)
                                    .await
                                && let Err(err) = run_request(&mut self, request).await
                            {
                                error!(
                                    "user {}: the executor loop is ending: {err:?}",
                                    self.user_id
                                );
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
}

/// Sleep for `remaining` as a branch of the biased select (spec §4.6;
/// #558 §3.7): a request (write, read, hydration, admin stats) arriving
/// mid-sleep is returned the moment it is ready (a backoff delays Strava
/// events, never the user's requests — a read is never delayed by the
/// Strava `Stop` nor by the `DbBackoff`); a fired wake (spec
/// §4.4/§4.6) interrupts the sleep with `None`, so the loop falls through
/// to the top and re-probes the queue at once; the sleep expiring (or the
/// channel closing — `None`, the shutdown corner) resolves with `None`.
pub(crate) async fn sleep_or_take_request(
    rx: &mut RequestReceiver,
    remaining: Duration,
    wake: &mut (impl Future<Output = ()> + Unpin),
) -> Option<Request> {
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
async fn queue_probe<T>(ctx: &mut LoopContext<T>) -> Option<Probe>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    let user_id = ctx.user_id;
    let mut conn = match ctx.source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            warn!(
                "user {}: the queue probe could not begin a transaction: {err:?}",
                user_id
            );
            return Some(Probe::Error);
        }
    };
    let probe = match get_event(user_id, &mut conn).await {
        Ok(QueueRead::Event(event)) => Probe::Event(event),
        Ok(QueueRead::Limited { until }) => Probe::Limited(until),
        Ok(QueueRead::Empty) => Probe::Empty,
        Err(err) => {
            warn!("user {}: the queue probe failed: {err:?}", user_id);
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
            user_id
        );
        return Some(Probe::Error);
    }
    ctx.backoff.reset();
    debug!("queue probe: {probe:?}");
    Some(probe)
}

/// One in-memory request in the loop's own transaction (spec #558 §3.2):
/// the variant decides the work — the write's `exec` dispatch, the read's
/// `apiread` dispatch, the hydration write, the admin stats read — and the
/// backoff discipline (the writes and the hydration record DB failures;
/// the reads never arm the backoff, spec #558 §3.7). The oneshot always
/// resolves and the message is consumed; only a failed `rollback` ends the
/// loop (the sub-dispatchers' `Err` return).
pub(crate) async fn run_request<T>(ctx: &mut LoopContext<T>, request: Request) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    match request {
        Request::Write {
            write,
            session: mut req_session,
            reply,
        } => run_write(ctx, &mut *req_session, write, reply).await,
        Request::Read {
            read,
            session: mut req_session,
            reply,
        } => run_read(ctx, &mut *req_session, read, reply).await,
        Request::Gear {
            session: mut req_session,
            reply,
        } => run_gear(ctx, &mut *req_session, reply).await,
        Request::AllStats {
            session: mut req_session,
            reply,
        } => run_all_stats(ctx, &mut *req_session, reply).await,
    }
}

/// The shared transaction shape of the four request runners (spec #558
/// §3.6): `begin` → `work` → `commit`, or `rollback` on `work`'s failure;
/// the oneshot always resolves and the message is consumed.
///
/// The two disciplines that differ per variant:
///
/// - `record_db_failure` — a `begin`/`commit` failure records it on the
///   in-memory backoff for the writes and the hydration (spec §4.6); the
///   reads never arm the backoff (spec #558 §3.7 — the queue probe is the
///   DB health monitor, and read traffic must not push the 30s cap up).
///   Either way, a transaction that completes cleanly (committed, or
///   rolled back after a domain error) resets the backoff, and only a
///   failed `rollback` ends the loop (the `Err` return).
/// - `frame` — a committed outcome the closure maps to a `Summary` pushes
///   one SSE frame to the user's streams before the oneshot reply; a
///   `None` pushes nothing (a read produces no `Summary`, spec #558 §3.6).
///
/// `noun` names the variant in the transaction log lines ("write", "read",
/// "hydration", "stats read"); `what` names it in the final failure line
/// ("API write", "API read", "hydration write", "all-user stats read").
///
/// `work` takes the connection by value and yields it back with the
/// outcome, so the commit/rollback (which consume the connection) stay in
/// this helper.
#[allow(clippy::too_many_arguments)] // the shape's eight degrees of freedom
async fn run_message<T, Out, Fut>(
    ctx: &mut LoopContext<T>,
    noun: &str,
    what: &str,
    reply: oneshot::Sender<TbResult<Out>>,
    record_db_failure: bool,
    shop: Option<ShopId>,
    frame: impl for<'a> Fn(&'a Out) -> Option<&'a Summary>,
    work: impl FnOnce(T::Conn) -> Fut,
) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
    Fut: Future<Output = (TbResult<Out>, T::Conn, Option<UserId>)>,
{
    let conn = match ctx.source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            // A `begin` failure: resolve the oneshot with the error and
            // continue — the message is consumed (the client is the retry
            // layer), the loop stays up.
            error!("could not begin the {noun}'s transaction: {err:?}");
            let _ = reply.send(Err(Error::AnyFailure(err.into())));
            if record_db_failure {
                ctx.backoff.record_failure();
            }
            return Ok(());
        }
    };

    let (outcome, conn, part_owner) = work(conn).await;
    let outcome: TbResult<Out> = match outcome {
        Ok(outcome) => match conn.commit().await {
            Ok(()) => {
                // A cleanly committed transaction is a healthy one: reset
                // the backoff.
                ctx.backoff.reset();
                Ok(outcome)
            }
            Err(err) => {
                // A `commit` failure: resolve the oneshot with the error
                // and continue.
                error!("committing the {noun}'s transaction failed: {err:?}");
                if record_db_failure {
                    ctx.backoff.record_failure();
                }
                Err(Error::AnyFailure(err.into()))
            }
        },
        Err(err) => {
            // A domain error: roll back and resolve the oneshot with it.
            // The transaction ran to completion, so the database is
            // healthy: reset the backoff.
            if let Err(rollback_err) = conn.rollback().await {
                error!("rolling back the {noun}'s transaction failed: {rollback_err:?}");
                let _ = reply.send(Err(Error::AnyFailure(rollback_err.into())));
                return Err(lifecycle_error(format_args!(
                    "rolling back the {noun}'s transaction failed"
                )));
            }
            ctx.backoff.reset();
            Err(err)
        }
    };
    if let Ok(outcome) = &outcome {
        // The frame (if the variant has one) is published on commit, before
        // the oneshot reply, routed by the session's context (spec #517,
        // ticket #584).
        if let Some(frame) = frame(outcome) {
            deliver_frame(
                &*ctx.pusher,
                &ctx.frames,
                ctx.user_id,
                shop,
                part_owner,
                frame,
            )
            .await;
        }
    }
    if let Err(err) = &outcome {
        error!("the {what} failed: {err:?}");
    }
    let _ = reply.send(outcome);
    Ok(())
}

/// One `ApiWrite` in one transaction (spec §4.3): `begin` → [`exec`] →
/// `commit`, or `rollback` on failure; the oneshot resolves with the
/// outcome either way, and a successful, non-empty `Summary` pushes one
/// frame to the user's SSE streams.
///
/// The onboarding cutover's Strava side (spec #558 §5.1): one message = one
/// loop-owned transaction containing the message's entire work — the
/// `UserOnboardingSync`'s transaction does the domain half (`exec`) and the
/// `insert_sync` in the same transaction, so the queued event commits with
/// the domain half (a failed write leaves neither behind).
///
/// Fail fast (spec §6.2, §4.6): the oneshot always resolves, the message is
/// consumed, and the loop continues — the client is the retry layer. A
/// `begin` or `commit` failure is a **DB failure**: the oneshot 500s, the
/// backoff records it, the loop continues (it does **not** end). Only a
/// failed `rollback` ends the loop with `Err` (the transaction's fate is not
/// ours to reason about); any other continued path proves the database
/// healthy and resets the backoff.
async fn run_write<T>(
    ctx: &mut LoopContext<T>,
    session: &mut dyn Session,
    write: ApiWrite,
    reply: oneshot::Sender<TbResult<WriteOutcome>>,
) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    debug!("user {}: Processing {write:?}", session.user_id());
    // The onboarding cutover's Strava side (spec #558 §5.1): extracted
    // before `exec` consumes the write, added to the same transaction.
    let onboarding_time = match &write {
        ApiWrite::UserOnboardingSync { time } => Some(*time),
        _ => None,
    };
    let shop = session.shop();
    run_message(
        ctx,
        "write",
        "API write",
        reply,
        true,
        shop,
        // The frame is the write's `Summary` (the descend's report is the
        // response body, not state — it never rides the frame).
        |outcome| Some(outcome.summary()),
        |mut conn| async move {
            let outcome = async {
                let outcome = exec(write, session, &mut conn).await?;
                if let Some(time) = onboarding_time {
                    insert_sync(session.user_id(), time, false, &mut conn).await?;
                }
                Ok(outcome)
            }
            .await;
            // In a shop context, resolve the part owner for the `push_to_user`
            // target (the rare case: only shop operations need it). The lookup
            // runs in the same transaction (before commit), so it sees the
            // operation's effects.
            let part_owner = if shop.is_some() {
                if let Some(part_id) = outcome
                    .as_ref()
                    .ok()
                    .and_then(|o| o.summary().referenced_part())
                {
                    part_id.owner(&mut conn).await.ok()
                } else {
                    None
                }
            } else {
                None
            };
            (outcome, conn, part_owner)
        },
    )
    .await
}

/// One `ApiRead` in the loop's own transaction (spec #558 §3.6/§3.7):
/// `begin` → [`tb_domain::read`] → `commit`, **read-only within it by spec** — a read
/// computes and returns data, and it pushes **no** SSE frame (it produces
/// no `Summary`).
///
/// The backoff discipline is the lane's, not a gate (spec #558 §3.7): a
/// read is never *delayed* by the Strava `Stop` nor the `DbBackoff` (both
/// sleeps are biased-select branches the read interrupts, like a write),
/// and a failed read **never arms** the backoff — a `begin`/transaction
/// failure resolves the oneshot with the `Err` (the route maps it to a
/// 4xx/5xx) and the loop continues with no `record_failure()`: the queue
/// probe is the DB health monitor, and read traffic (e.g. a reconnecting
/// client's burst) must not push the 30s cap up. Symmetrically, a read
/// transaction that completes cleanly (committed, or rolled back after a
/// domain error) **resets** the backoff, per the existing "any healthy
/// transaction resets" rule. Only a failed `rollback` ends the loop.
async fn run_read<T>(
    ctx: &mut LoopContext<T>,
    session: &mut dyn Session,
    read: ApiRead,
    reply: oneshot::Sender<TbResult<ReadOutcome>>,
) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    debug!("user {}: Reading {read:?}", session.user_id());
    run_message(
        ctx,
        "read",
        "API read",
        reply,
        false,          // a failed read never arms the backoff (spec #558 §3.7)
        session.shop(), // a read pushes no frame; the context is unused
        |_| None,       // a read pushes no frame (spec #558 §3.6)
        |mut conn| async move {
            let outcome = tb_domain::read(read, session, &mut conn).await;
            (outcome, conn, None)
        },
    )
    .await
}

/// The hydration write in the loop's own transaction (spec #558 §4):
/// `begin` → [`StravaUser::update_gear`] → read the parts it created →
/// `commit`. `update_gear` is a `tb_strava` op (the session's
/// `request_json` plus the `into_partid` store work), so it is dispatched
/// loop-side — not an `ApiWrite` variant.
///
/// The frame is uniform (like every other write): the hydrate's `Summary`
/// is the **union of the parts it created** — one entry per part, empty
/// when nothing is new (the common case: `push_frame` sends nothing) — and
/// it is published on commit, before the oneshot reply (the client already
/// buffers frames that race the snapshot). The backoff discipline is the
/// write's: a `begin`/`commit` failure records it, a completed transaction
/// (committed, or rolled back after an error) resets it, and only a failed
/// `rollback` ends the loop.
async fn run_gear<T>(
    ctx: &mut LoopContext<T>,
    session: &mut dyn Session,
    reply: oneshot::Sender<TbResult<Summary>>,
) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    debug!("user {}: Hydrating the user's gear", session.user_id());
    // The hydration's work in the loop's transaction: import the Strava
    // gear, then read back the parts it created — the frame is their union
    // (one entry per part, empty when nothing is new, spec #558 §4).
    let mut summary = Summary::default();
    let shop = session.shop();
    run_message(
        ctx,
        "hydration",
        "hydration write",
        reply,
        true,
        shop,
        |summary| Some(summary),
        |mut conn| async move {
            let outcome = async {
                let created = StravaUser::update_gear(session, &mut conn).await?;
                for id in created {
                    summary.parts.upsert(id.part(session, &mut conn).await?);
                }
                Ok(())
            }
            .await;
            (outcome.map(|()| summary), conn, None)
        },
    )
    .await
}

/// The admin `AllStats` read in the loop's own transaction (spec #558
/// §3.3): `begin` → [`get_all_stats`] → `commit`, read-only within it by
/// spec. `get_all_stats` is a `tb_strava` op, so it is dispatched loop-side
/// — the same home the `Gear` hydration and the Strava events' `process`
/// get — riding the admin user's channel. The backoff discipline is the
/// read's (spec #558 §3.7): a failure resolves the oneshot with the `Err`
/// and never arms the backoff; a cleanly committed read resets it.
async fn run_all_stats<T>(
    ctx: &mut LoopContext<T>,
    session: &mut dyn Session,
    reply: oneshot::Sender<TbResult<Vec<StravaStat>>>,
) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    debug!("user {}: Reading the all-user stats", session.user_id());
    let shop = session.shop();
    run_message(
        ctx,
        "stats read",
        "all-user stats read",
        reply,
        false, // a failed read never arms the backoff (spec #558 §3.7)
        shop,
        |_| None, // the stats read pushes no frame (spec #558 §3.7)
        |mut conn| async move {
            let outcome = get_all_stats(&mut conn).await;
            (outcome, conn, None)
        },
    )
    .await
}

/// The `Err` value `run` returns when a message's transaction lifecycle
/// fails: a marker distinct from any domain error of the message itself. The
/// original error was already handed to the message's reply (a failed write
/// must 500 the client), so what ends the loop is a fresh marker. The web
/// layer treats an `Err` from `run` like a panic (close the streams, let the
/// client reconnect and respawn); the original is named in the log.
fn lifecycle_error(what: impl std::fmt::Display) -> Error {
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
async fn run_event<T>(ctx: &mut LoopContext<T>, event: &Event) -> Result<(), Error>
where
    T: TxnSource + Clone + Send + Sync + 'static,
{
    debug!("user {}: Processing {event}", ctx.user_id);

    let mut conn = match ctx.source.begin().await {
        Ok(conn) => conn,
        Err(err) => {
            // DB failure (spec §4.6): back off, continue — the event stays
            // queued and is retried after the pause.
            warn!("could not begin a transaction for the event: {err:?}");
            ctx.backoff.record_failure();
            return Ok(());
        }
    };

    // The identity-only session the domain event processing needs, built at
    // the last moment (the event is the user's own, so the owner is
    // `user_id` and no shop context applies).
    let mut session = crate::registry::ExecutorSession::new(ctx.user_id);
    match process(&mut session, &mut conn).await {
        Ok(summary) => match conn.commit().await {
            Ok(()) => {
                // Route the event's `Summary`: a Strava event is the user's
                // own parts, so no shop context (spec #517, ticket #584) —
                // the executor's own personal stream. Then announce the
                // consumed event (spec §6.4) for good, empty summary or not.
                deliver_frame(&*ctx.pusher, &ctx.frames, ctx.user_id, None, None, &summary).await;
                let _ = ctx.events.send(event.clone());
                ctx.backoff.reset();
                Ok(())
            }
            Err(err) => {
                // DB failure (spec §4.6): back off, continue. The event is
                // either committed (the re-probe finds the next one) or still
                // queued (the re-probe finds it again) — either way the retry
                // after the pause settles it.
                error!("committing the event's transaction failed: {err:?}");
                ctx.backoff.record_failure();
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
            ctx.backoff.record_failure();
            Ok(())
        }
    }
}

#[cfg(test)]
mod executor_tests;
