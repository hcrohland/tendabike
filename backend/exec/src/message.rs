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

//! The executor loop's messages and in-memory plumbing (ADR-0005, executable
//! spec #446 §4.2).
//!
//! The per-user executor consumes two message sources, unified in
//! [`Message`]:
//!
//! - **API writes** — an in-memory channel, one per user. Every mutating
//!   route enqueues an [`ApiWriteRequest`] (the [`ApiWrite`] plus a oneshot
//!   reply) and awaits the reply; within the channel, writes process FIFO.
//! - **Strava events** — the existing `strava_events` database queue, read
//!   latest-per-object (`tb_strava::event::get_event`).
//!
//! The loop also owns the SSE push sink: a `broadcast` channel of
//! [`Summary`] frames, one frame per **non-empty** completed message
//! (spec §4.5 — an empty `Summary` pushes nothing; a dropped frame is
//! non-fatal, the client's reconnect + full refresh covers it).

use tb_domain::{ApiWrite, Summary, TbResult};
use tb_strava::event::Event;
use tokio::sync::{broadcast, mpsc, oneshot};

/// The one message the per-user executor consumes: a unified wrapper over the
/// two sources (spec §4.2).
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// An in-memory write from the user's API channel.
    ApiWrite(ApiWrite),
    /// An event read from the user's `strava_events` queue.
    Strava(Event),
}

/// One API write as the executor receives it: the [`ApiWrite`] plus a
/// oneshot the mutating route awaits for the write's outcome (spec §6.2:
/// enqueue + await; success resolves the `Summary`, failure resolves the
/// error and the route maps it to a 4xx/5xx).
///
/// `pub` (fields too) because the web layer builds one per request and the
/// executor consumes it; the oneshot keeps the type from deriving `Clone`
/// or `PartialEq` — those live on the [`ApiWrite`] the request bundles.
pub struct ApiWriteRequest {
    /// The write to apply.
    pub write: ApiWrite,
    /// Resolved exactly once when the write is committed or fails.
    pub reply: oneshot::Sender<TbResult<Summary>>,
}

impl ApiWriteRequest {
    /// Pairs a write with a fresh reply channel: the request goes on the
    /// user's executor channel, the receiver is awaited by the route.
    pub fn new(write: ApiWrite) -> (Self, oneshot::Receiver<TbResult<Summary>>) {
        let (reply, rx) = oneshot::channel();
        (Self { write, reply }, rx)
    }
}

impl std::fmt::Debug for ApiWriteRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiWriteRequest")
            .field("write", &self.write)
            .field("reply", &"<oneshot>")
            .finish()
    }
}

/// The send half of a user's API-write channel (held by the web layer).
pub type ApiWriteSender = mpsc::UnboundedSender<ApiWriteRequest>;

/// The receive half of a user's API-write channel (consumed by the loop).
pub type ApiWriteReceiver = mpsc::UnboundedReceiver<ApiWriteRequest>;

/// A per-user API-write channel, as the web layer constructs one for a user's
/// executor (spec §4.2).
///
/// Unbounded on purpose: enqueuing a write must never block the route — a
/// write queued behind a slow in-progress message just waits on its oneshot
/// (the wait is unbounded, per spec §6.2); per-user serialization is the
/// single loop, not channel backpressure.
pub fn api_write_channel() -> (ApiWriteSender, ApiWriteReceiver) {
    mpsc::unbounded_channel()
}

/// Push one completed message's [`Summary`] to the user's SSE streams
/// (spec §4.5): a frame for every **non-empty** summary, none for an empty
/// one. A send that finds no receiver (or a lagged one) is dropped without
/// error — a lost frame just means a not-yet-connected or slow client, and
/// the client's reconnect + full refresh covers it.
pub fn push_frame(frames: &broadcast::Sender<Summary>, summary: &Summary) {
    if summary.is_empty() {
        return;
    }
    let _ = frames.send(summary.clone());
}
