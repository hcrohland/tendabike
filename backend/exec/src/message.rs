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
//! spec #446 §4.2; ADR-0007, executable spec #558 §3.2).
//!
//! The per-user executor consumes two message sources:
//!
//! - **API requests** — an in-memory channel, one per user. Every read and
//!   mutating route enqueues a [`Request`] (the operation plus a oneshot
//!   reply) and awaits the reply; within the channel, requests process in
//!   **FIFO total order per user** (spec #558 §3.2: a read enqueued after a
//!   write sees that write — the ordering contract a second driver gets).
//! - **Provider events** — the provider's event database queue, read
//!   latest-per-object through the lane's `probe`.
//!
//! The loop also owns the SSE push sink: a `broadcast` channel of
//! [`Summary`] frames, one frame per **non-empty** completed message
//! (spec §4.5 — an empty `Summary` pushes nothing; a dropped frame is
//! non-fatal, the client's reconnect + full refresh covers it).

#[cfg(test)]
use crate::facade::Summary;
#[cfg(test)]
use tokio::sync::broadcast;
use tokio::sync::{mpsc, oneshot};

use crate::facade::{ApiRead, ApiWrite, ReadOutcome, Session, TbResult, WriteOutcome};

/// The one message the per-user executor consumes: a unified wrapper over the
/// message sources (spec §4.2) — the user's API-write channel plus the
/// provider event queue.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Message {
    /// An in-memory write from the user's API channel.
    ApiWrite(ApiWrite),
    /// An event read from the user's provider event queue.
    Event(()),
}

/// One API request as the executor receives it: the unified per-user
/// message (spec #558 §3.2), replacing the bare write request. One variant
/// per message type the routes enqueue, each carrying its oneshot reply —
/// the route awaits it (unbounded, spec #558 §3.7), and the loop resolves
/// it exactly once when the message commits or fails:
///
/// - [`Request::Write`] — a mutating route's [`ApiWrite`]: success resolves
///   the `WriteOutcome` (the `Summary` — the frame — plus, for the descend,
///   its match report, the response body), failure resolves the error and
///   the route maps it to a 4xx/5xx.
/// - [`Request::Read`] — a read route's [`ApiRead`] descriptor: the loop
///   runs it in its own transaction (read-only within it by spec) and
///   resolves the read's data (a [`ReadOutcome`]) or the error.
///
/// `pub` (the fields stay private) because the integration (seam) suite —
/// a separate crate — builds one per request through the constructors and
/// the executor consumes it; the oneshots keep the type from deriving
/// `Clone` or `PartialEq` — those live on the payloads the request bundles.
pub enum Request {
    /// An API write from the user's channel.
    Write {
        /// The write to apply.
        write: ApiWrite,
        session: Box<dyn Session>,
        /// Resolved exactly once when the write is committed or fails.
        reply: oneshot::Sender<TbResult<WriteOutcome>>,
    },
    /// An API read from the user's channel, run in the loop's own
    /// transaction (read-only within it by spec).
    Read {
        /// The read to apply.
        read: ApiRead,
        session: Box<dyn Session>,
        /// Resolved exactly once when the read commits or fails.
        reply: oneshot::Sender<TbResult<ReadOutcome>>,
    },
}

impl Request {
    /// Pairs a write and its session with a fresh reply channel: the request
    /// goes on the user's executor channel, the receiver is awaited by the
    /// route.
    pub fn new_write(
        session: Box<dyn Session>,
        write: ApiWrite,
    ) -> (Self, oneshot::Receiver<TbResult<WriteOutcome>>) {
        let (reply, rx) = oneshot::channel();
        (
            Self::Write {
                write,
                session,
                reply,
            },
            rx,
        )
    }

    /// Pairs a read and its session with a fresh reply channel: the request
    /// goes on the user's executor channel, the receiver is awaited by the
    /// route.
    pub fn new_read(
        session: Box<dyn Session>,
        read: ApiRead,
    ) -> (Self, oneshot::Receiver<TbResult<ReadOutcome>>) {
        let (reply, rx) = oneshot::channel();
        (
            Self::Read {
                read,
                session,
                reply,
            },
            rx,
        )
    }
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Write { write, .. } => f
                .debug_struct("Request::Write")
                .field("write", write)
                .field("session", &"<session>")
                .field("reply", &"<oneshot>")
                .finish(),
            Self::Read { read, .. } => f
                .debug_struct("Request::Read")
                .field("read", read)
                .field("session", &"<session>")
                .field("reply", &"<oneshot>")
                .finish(),
        }
    }
}

/// The send half of a user's request channel (held by the registry).
pub(crate) type RequestSender = mpsc::UnboundedSender<Request>;

/// The receive half of a user's request channel (consumed by the loop).
pub type RequestReceiver = mpsc::UnboundedReceiver<Request>;

/// A per-user request channel, as the web layer constructs one for a user's
/// executor (spec #446 §4.2; #558 §3.2).
///
/// Unbounded on purpose: enqueuing a request must never block the route — a
/// request queued behind a slow in-progress message just waits on its
/// oneshot (the wait is unbounded, per spec #558 §3.7); per-user
/// serialization is the single loop, not channel backpressure.
pub fn request_channel() -> (RequestSender, RequestReceiver) {
    mpsc::unbounded_channel()
}

/// Push one completed message's [`Summary`] to the user's SSE streams
/// (spec §4.5): a frame for every **non-empty** summary, none for an empty
/// one. A send that finds no receiver (or a lagged one) is dropped without
/// error — a lost frame just means a not-yet-connected or slow client, and
/// the client's reconnect + full refresh covers it.
#[cfg(test)]
pub(crate) fn push_frame(frames: &broadcast::Sender<Summary>, summary: &Summary) {
    if summary.is_empty() {
        return;
    }
    let _ = frames.send(summary.clone());
}
