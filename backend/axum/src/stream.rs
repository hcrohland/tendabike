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
    extract::{Query, State},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::unfold;
use http::StatusCode;
use log::error;
use tokio::sync::{broadcast, watch};

use tb_domain::{Session, ShopId, Summary, UserId};
use tb_exec::{ExecutorStatus, HEARTBEAT_INTERVAL};

use crate::appstate::AppState;
use crate::error::AppError;
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

/// One shop-stream connection's `unfold` (spec #517, ticket #583): the current
/// frame subscription on the shop's standalone `broadcast` channel. A `None`
/// frame subscription means "subscribe on the next step". Unlike the personal
/// stream there is no `ExecutorStatus` — the shop channel is not tied to an
/// executor's lifecycle, so there is no reap-to-reconnect to handle.
struct ShopStreamSeed {
    app: AppState,
    user_id: UserId,
    shop_id: ShopId,
    rx: Option<broadcast::Receiver<Summary>>,
}

/// One step of the shop stream: ensure a live subscription on the shop's
/// channel, then forward the next frame. A frame is the `Summary` JSON (no
/// envelope, no sequence). The shop channel has no executor fate to watch:
/// when it closes (the shop was deleted) the stream ends so the client
/// reconnects and re-hydrates.
async fn next_shop_event(seed: &mut ShopStreamSeed) -> Option<Result<Event, Infallible>> {
    let mut rx = match seed.rx.take() {
        Some(rx) => rx,
        None => match seed
            .app
            .api
            .subscribe_shop(seed.user_id, seed.shop_id)
            .await
        {
            Ok(rx) => rx,
            Err(err) => {
                error!(
                    "opening the shop stream for user {} (shop {}) failed: {err:?}",
                    seed.user_id, seed.shop_id
                );
                return None;
            }
        },
    };

    match rx.recv().await {
        Ok(summary) => {
            seed.rx = Some(rx);
            Some(Ok(frame_event(&summary)))
        }
        // The shop channel closed (the shop was deleted): end the stream so
        // the client reconnects (its next `subscribe_shop` answers 404).
        Err(_) => None,
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

/// The `?shop=` query on the stream route (spec #517, ticket #583): present
/// means the shop stream (owner-only), absent means the personal stream.
#[derive(serde::Deserialize)]
pub(crate) struct ShopQuery {
    shop: Option<ShopId>,
}

/// `GET /api/user/stream` (spec §6.1) — and `?shop=<id>` (spec #517, ticket
/// #583): session-scoped, next to `/api/user/summary` (the session extractor
/// rejects a missing session with 401). Without the `?shop=` parameter, the
/// personal stream: spawn/get the user's executor, register this connection's
/// stream, and forward `Summary` frames (heartbeats while the executor is
/// reaped). With `?shop=<id>`, the shop stream: open the shop's standalone
/// channel (owner-only — a non-owner is `Forbidden`, mapped to a 403) and
/// forward its frames. Teardown (deregister; the stream ends once the
/// connection is dropped) happens when the stream is dropped.
pub(crate) async fn endpoint(
    user: RequestSession,
    State(state): State<AppState>,
    Query(ShopQuery { shop }): Query<ShopQuery>,
) -> Response {
    let user_id = user.user_id();
    match shop {
        Some(shop_id) => shop_endpoint(user_id, state, shop_id).await,
        None => personal_endpoint(user_id, state).await,
    }
}

/// The personal stream (spec §6.1): spawn/get the user's executor, register
/// this connection's stream, and forward `Summary` frames (heartbeats while
/// the executor is reaped). Teardown (deregister; the executor reaps once
/// this was the last stream — no streams attached is the only reap rule)
/// happens when the stream is dropped — the broadcast receiver's count is the
/// loop's stream-count signal.
async fn personal_endpoint(user_id: UserId, state: AppState) -> Response {
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

/// The shop stream (spec #517, ticket #583): open the shop's standalone
/// channel (owner-only — a non-owner is `Forbidden`, mapped to a 403) and
/// forward its frames. The shop channel has no executor fate, so the stream
/// ends when the channel closes (the shop was deleted).
async fn shop_endpoint(user_id: UserId, state: AppState, shop_id: ShopId) -> Response {
    let rx = match state.api.subscribe_shop(user_id, shop_id).await {
        Ok(rx) => rx,
        Err(err) => return AppError::from(err).into_response(),
    };

    let stream = unfold(
        ShopStreamSeed {
            app: state,
            user_id,
            shop_id,
            rx: Some(rx),
        },
        |mut seed| async move { next_shop_event(&mut seed).await.map(|item| (item, seed)) },
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(HEARTBEAT_INTERVAL).text("ping"))
        .into_response()
}
