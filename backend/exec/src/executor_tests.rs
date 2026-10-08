//! Pure-brain unit tests for the executor loop's control flow (ADR-0005,
//! executable spec #446 §10: no store fakes, no database — the loop's
//! message selection, reclaim decision, and frame construction are structured
//! as units testable with a channel and dummy futures; the transaction seam
//! is verified against Postgres in `tb_sqlx`'s `tests/executor_seam.rs`).

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use tb_domain::{ApiWrite, Summary, TbResult, Usage};
use tb_strava::event::Event;
use tokio::sync::{broadcast, oneshot};

use crate::executor::{Probe, Select, select_message};
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

fn request() -> (ApiWriteRequest, oneshot::Receiver<TbResult<Summary>>) {
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
    let sel = select_message(rx.recv(), ready_probe(Probe::Event(Event::default()))).await;
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
    let sel = select_message(rx.recv(), ready_probe(Probe::Event(Event::default()))).await;
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
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty)).await;
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
    let sel = select_message(rx.recv(), ready_probe(Probe::Empty)).await;
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
    let sel = select_message(rx.recv(), ready_probe(Probe::Error)).await;
    assert!(
        matches!(sel, Select::StravaError),
        "a failed probe is StravaError, got {sel:?}"
    );
}

#[tokio::test]
async fn select_waits_for_the_write_over_a_slow_queue() {
    // The queue probe is pending (slow); a write that arrives while waiting is
    // taken the moment it is ready — the write never waits behind the probe.
    let (tx, mut rx) = api_write_channel();
    let sel = tokio::spawn(async move {
        let _ = tx.send(request().0);
        select_message(rx.recv(), Never).await
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
