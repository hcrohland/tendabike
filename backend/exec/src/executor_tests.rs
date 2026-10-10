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

//! Pure-brain unit tests for the executor loop's control flow (ADR-0005,
//! executable spec #446 §10: no store fakes, no database — the loop's
//! message selection, reclaim decision, and frame construction are structured
//! as units testable with a channel and dummy futures; the transaction seam
//! is verified against Postgres in `tb_sqlx`'s `tests/executor_seam.rs`).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tb_domain::{ApiWrite, Summary, TbResult, Usage, WriteOutcome};
use tb_strava::event::Event;
use tokio::sync::{broadcast, oneshot};

use crate::executor::{DbBackoff, Probe, Select, select_message, stop_remaining};
use crate::message::{ApiWriteRequest, push_frame};
use crate::{Action, Message, api_write_channel, next_action};

/// A future that never resolves: stands in for the Strava queue probe being
/// blocked (a slow or busy queue), so the select has a second, non-ready
/// branch without a store.
struct Never;

impl Future for Never {
    type Output = Option<Probe>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

/// A future that immediately resolves with a ready probe outcome.
async fn ready_probe(probe: Probe) -> Option<Probe> {
    Some(probe)
}

fn request() -> (ApiWriteRequest, oneshot::Receiver<TbResult<WriteOutcome>>) {
    ApiWriteRequest::new(ApiWrite::UserOnboardingPostpone)
}

// --- `next_action`: the reclaim decision ---

#[test]
fn next_action_prefer_write_over_everything() {
    // A client action must never wait behind a queued event, active streams,
    // or an elapsed idle window (spec §4.3: the select is biased).
    assert_eq!(
        next_action(
            true,
            true,
            3,
            Duration::from_secs(999),
            Duration::from_secs(1)
        ),
        Action::ProcessWrite
    );
}

#[test]
fn next_action_strava_when_no_write() {
    assert_eq!(
        next_action(false, true, 0, Duration::ZERO, Duration::from_secs(1)),
        Action::ProcessStrava
    );
}

#[test]
fn next_action_reap_with_no_streams() {
    // Nothing pending and no one listening: exit, whatever the idle clock says.
    assert_eq!(
        next_action(false, false, 0, Duration::ZERO, Duration::from_secs(60)),
        Action::Reap
    );
    assert_eq!(
        next_action(
            false,
            false,
            0,
            Duration::from_secs(3600),
            Duration::from_secs(60)
        ),
        Action::Reap
    );
}

#[test]
fn next_action_sleep_while_streams_listen() {
    // Streams still attached and the idle window not over: stay alive.
    assert_eq!(
        next_action(
            false,
            false,
            1,
            Duration::from_secs(10),
            Duration::from_secs(60)
        ),
        Action::Sleep
    );
}

#[test]
fn next_action_reap_after_idle_timeout() {
    // Streams attached but the idle timeout elapsed with no messages: exit —
    // the streams keep heartbeating (web layer), the next write or connect
    // respawns the loop (spec §4.4).
    assert_eq!(
        next_action(
            false,
            false,
            2,
            Duration::from_secs(60),
            Duration::from_secs(60)
        ),
        Action::Reap
    );
}

// --- `select_message`: the biased select ---

#[tokio::test]
async fn select_api_write_wins_over_ready_strava() {
    // Both ready at the same instant: the write branch is polled first.
    let (_tx, mut rx) = api_write_channel();
    let (req, _reply) = request();
    let _ = _tx.send(req);
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(
        rx.recv(),
        ready_probe(Probe::Event(Event::default())),
        &mut wake_fut,
    )
    .await;
    assert!(
        matches!(sel, Select::ApiWrite(ref req) if matches!(req.write, ApiWrite::UserOnboardingPostpone)),
        "the ready API write must win the biased select, got {sel:?}"
    );
}

#[tokio::test]
async fn select_strava_when_no_write_pending() {
    // An empty-but-open channel parks the write branch; the ready Strava
    // branch is then taken.
    let (_tx, mut rx) = api_write_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(
        rx.recv(),
        ready_probe(Probe::Event(Event::default())),
        &mut wake_fut,
    )
    .await;
    assert!(
        matches!(sel, Select::Strava(ref e) if *e == Event::default()),
        "a ready Strava event is taken when no write is pending, got {sel:?}"
    );
}

#[tokio::test]
async fn select_idle_when_queue_empty_and_no_write() {
    // Neither branch ready-yet-interesting: an empty queue (committed probe)
    // with no pending write is the idle state the reclaim policy decides on.
    let (_tx, mut rx) = api_write_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Idle),
        "empty queue + no write is Idle, got {sel:?}"
    );
}

#[tokio::test]
async fn select_idle_when_channel_closed_and_queue_empty() {
    // A dropped sender resolves `recv()` with `None`; with an empty queue the
    // select is idle (in production the sender lives for the process, so this
    // is a shutdown corner, not a normal state).
    let (tx, mut rx) = api_write_channel();
    drop(tx);
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Idle),
        "closed channel + empty queue is Idle, got {sel:?}"
    );
}

#[tokio::test]
async fn select_strava_error_is_its_own_state() {
    // A failed probe is reported, not silently read as "empty": the loop
    // pauses instead of deciding the queue drained.
    let (_tx, mut rx) = api_write_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Error), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::StravaError),
        "a failed probe is StravaError, got {sel:?}"
    );
}

#[tokio::test]
async fn select_limited_when_stop_active() {
    // An active global rate-limit Stop is its own select state (spec #446
    // §4.6): the loop sleeps until the deadline as a branch of the select,
    // interruptible by API writes.
    let (_tx, mut rx) = api_write_channel();
    let wake = tokio::sync::Notify::new();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = select_message(rx.recv(), ready_probe(Probe::Limited(1234)), &mut wake_fut).await;
    assert!(
        matches!(sel, Select::Limited(1234)),
        "an active Stop is Select::Limited, got {sel:?}"
    );
}

// --- `stop_remaining`: the Stop deadline as a sleep duration ---

#[test]
fn stop_remaining_until_the_deadline() {
    assert_eq!(stop_remaining(1000, 400), Duration::from_secs(600));
}

#[test]
fn stop_remaining_expired_is_zero() {
    // An expired (or now-exactly-at) deadline sleeps nothing: the next probe
    // deletes the Stop and reads on.
    assert_eq!(stop_remaining(1000, 1000), Duration::ZERO);
    assert_eq!(stop_remaining(400, 1000), Duration::ZERO);
}

// --- `DbBackoff`: the in-memory DB-failure backoff (spec #446 §4.6) ---

#[test]
fn db_backoff_doubles_until_the_cap() {
    // Exponential, capped at 30s (spec #446 §4.6): a broken database must not
    // hot-spin the loop, and a recovering one is probed at human pace.
    let mut b = DbBackoff::new();
    assert_eq!(b.record_failure(), Duration::from_secs(1));
    assert_eq!(b.record_failure(), Duration::from_secs(2));
    assert_eq!(b.record_failure(), Duration::from_secs(4));
    assert_eq!(b.record_failure(), Duration::from_secs(8));
    assert_eq!(b.record_failure(), Duration::from_secs(16));
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(30),
        "16s x 2 caps at 30s"
    );
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(30),
        "and stays capped"
    );
}

#[test]
fn db_backoff_resets_after_a_success() {
    // A healthy database (any committed transaction) restarts the backoff at
    // its base: the next failure gets the short pause, not the cap.
    let mut b = DbBackoff::new();
    b.record_failure();
    b.record_failure();
    b.reset();
    assert_eq!(
        b.record_failure(),
        Duration::from_secs(1),
        "a success resets the backoff to its base"
    );
}

#[tokio::test]
async fn select_waits_for_the_write_over_a_slow_queue() {
    // The queue probe is pending (slow); a write that arrives while waiting is
    // taken the moment it is ready — the write never waits behind the probe.
    let (tx, mut rx) = api_write_channel();
    let sel = tokio::spawn(async move {
        let _ = tx.send(request().0);
        let wake = tokio::sync::Notify::new();
        let mut wake_fut = Box::pin(wake.notified());
        select_message(rx.recv(), Never, &mut wake_fut).await
    });
    // Give the task a moment to park on the never-ready queue probe; the send
    // in the same task has already happened, so the write branch is ready.
    let sel = tokio::time::timeout(Duration::from_secs(5), sel)
        .await
        .expect("a queued write resolves the select")
        .expect("the task does not panic");
    assert!(
        matches!(sel, Select::ApiWrite(_)),
        "a write arriving over a slow queue wins, got {sel:?}"
    );
}

#[tokio::test]
async fn select_wake_fired_before_the_select_is_wake() {
    // Permit semantics: a notify that fires before the `notified()` future is
    // created is not lost — the select resolves to the wake state (the loop
    // re-probes the queue on its next cycle), even with a slow probe parked.
    let (_tx, mut rx) = api_write_channel();
    let wake = tokio::sync::Notify::new();
    wake.notify_one();
    let mut wake_fut = Box::pin(wake.notified());
    let sel = tokio::time::timeout(
        Duration::from_secs(5),
        select_message(rx.recv(), Never, &mut wake_fut),
    )
    .await
    .expect("a pre-fired wake resolves the select");
    assert!(
        matches!(sel, Select::Wake),
        "a fired wake is the wake state, got {sel:?}"
    );
}

#[tokio::test]
async fn select_wake_firing_while_parked_is_wake() {
    // The queue probe is pending (slow) and no write is queued; a wake that
    // fires while the select is parked resolves it to the wake state — the
    // loop re-probes the queue on its next cycle.
    let (_tx, mut rx) = api_write_channel();
    let wake = std::sync::Arc::new(tokio::sync::Notify::new());
    let wake_in_task = wake.clone();
    let task = tokio::spawn(async move {
        let mut wake_fut = Box::pin(wake_in_task.notified());
        select_message(rx.recv(), Never, &mut wake_fut).await
    });
    // Let the task park on the never-ready probe, then fire the wake.
    tokio::time::sleep(Duration::from_millis(50)).await;
    wake.notify_one();
    let sel = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("a fired wake resolves the select")
        .expect("the task does not panic");
    assert!(
        matches!(sel, Select::Wake),
        "a wake firing mid-park is the wake state, got {sel:?}"
    );
}

// --- `Message` and the frame push ---

#[test]
fn message_carries_both_sources() {
    let write = ApiWrite::UserOnboardingPostpone;
    assert_eq!(Message::ApiWrite(write.clone()), Message::ApiWrite(write));
    assert_eq!(
        Message::Strava(Event::default()),
        Message::Strava(Event::default())
    );
}

#[test]
fn request_bundles_write_and_reply() {
    let (req, mut reply) = request();
    // Open, but nothing sent yet: the reply is pending, not closed.
    assert!(
        matches!(
            reply.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ),
        "the reply must be open"
    );
    let (tx, mut rx) = api_write_channel();
    tx.send(req)
        .expect("an unbounded channel accepts the request");
    let back = rx.try_recv().expect("the request round-trips the channel");
    assert_eq!(back.write, ApiWrite::UserOnboardingPostpone);
}

#[test]
fn frame_push_delivers_non_empty_summaries() {
    let (tx, mut rx) = broadcast::channel(4);
    let mut frame = Summary::default();
    let usage = Usage::default();
    frame.usages.insert(usage.id, Some(usage));

    push_frame(&tx, &frame);
    let got = rx
        .try_recv()
        .expect("a non-empty summary pushes exactly one frame");
    assert_eq!(got, frame);
}

#[test]
fn frame_push_skips_empty_summaries() {
    let (tx, mut rx) = broadcast::channel(4);
    push_frame(&tx, &Summary::default());
    assert!(
        rx.try_recv().is_err(),
        "an empty summary pushes nothing (spec §4.5)"
    );
}
