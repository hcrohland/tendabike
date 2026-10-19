//! The SSE stream endpoint (ADR-0005, executable spec #446 §6.1; ADR-0007,
//! executable spec #558 §6.4/§7).
//!
//! The web layer owns the SSE streams; the per-user executor task (the
//! `tb_exec` loop) is spawned on demand by the service API (the `Registry`
//! in `tb_exec` — the web layer names the trait, never the registry).
//!
//! Each SSE frame is the completed message's [`Summary`] (the map JSON,
//! spec §2) — no envelope, no type discriminator, no sequence number.
//! While the executor is reaped (idle), the stream stays alive on
//! heartbeat comments (`: ping`), invisible to the client's `onmessage`.
//!
//! The executor's fate is shared through a `watch` channel
//! ([`ExecutorStatus`] — named by the web layer in the stream teardown,
//! spec #558 §6.3): a clean reap keeps the connection alive (a heartbeat,
//! then re-subscribe); a dead executor ends the stream so the client
//! reconnects and respawns.

use std::convert::Infallible;

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
use tokio::sync::{broadcast, watch};

use tb_domain::{Session, Summary, UserId};
use tb_exec::{ExecutorStatus, HEARTBEAT_INTERVAL};

use crate::appstate::AppState;
use crate::strava::RequestSession;

/// One SSE connection's `unfold`: the current frame subscription
/// and its executor's fate channel. A `None` frame subscription means "the
/// executor settled; re-subscribe (respawning it) on the next step".
struct StreamSeed {
    app: AppState,
    user_id: UserId,
    rx: Option<broadcast::Receiver<Summary>>,
    status: Option<watch::Receiver<ExecutorStatus>>,
}

/// One step of the SSE stream: ensure a live subscription (re-subscribing
/// — and respawning the executor — after it settles), then forward the next
/// frame. A frame is the `Summary` JSON (no envelope, no sequence). A clean
/// reap (the connect-time race — the executor reaped before this
/// connection's receiver attached) yields a heartbeat and re-subscribes on
/// the next step; a dead executor ends the stream (the client reconnects
/// and respawns, spec §4.5).
async fn next_event(seed: &mut StreamSeed) -> Option<Result<Event, Infallible>> {
    let (mut rx, mut status) = match (seed.rx.take(), seed.status.take()) {
        (Some(rx), Some(status)) => (rx, status),
        _ => match seed.app.api.subscribe(seed.user_id).await {
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
/// Teardown (deregister; the executor reaps once this was the last stream —
/// no streams attached is the only reap rule) happens when the stream is
/// dropped — the broadcast receiver's count is the loop's stream-count
/// signal.
pub(crate) async fn endpoint(user: RequestSession, State(state): State<AppState>) -> Response {
    let user_id = user.user_id();
    let (rx, status) = match state.api.subscribe(user_id).await {
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
        .keep_alive(KeepAlive::new().interval(HEARTBEAT_INTERVAL).text("ping"))
        .into_response()
}
