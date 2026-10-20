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

//! The transaction seam of the per-user executor loop (ADR-0005, executable
//! spec #446 §4.1).
//!
//! The lifecycle is split across two types today — `DbPool::begin` opens a
//! transaction, `SqlxConn::commit`/`rollback` close it — and one trait cannot
//! carry methods on two different types, so the seam is two traits linked by
//! an associated type: a [`TxnSource`] whose `Conn` is a [`Txn`]. The
//! executor loop is generic over the source and names only the traits, never
//! a concrete adapter — the `StravaStore` bound sits on the associated
//! type, so every generic over the seam gets it for free.

use tb_domain::{Error, Store, TbResult};
use tb_strava::StravaStore;

/// A source of open transactions for the executor loop (ADR-0005, executable
/// spec #446 §4.1).
///
/// The loop is generic over this trait: it calls `begin` to open a
/// transaction for each message and never names the concrete source —
/// `tb_sqlx`'s `DbPool` satisfies it and is injected at the composition root
/// (`app`). `Send + Sync` because the loop is a long-lived `Send` task that
/// holds `&self` across awaits.
#[async_trait::async_trait]
pub trait TxnSource: Send + Sync {
    /// A connection with an open transaction: every [`Store`] sub-trait
    /// (so it can run the domain operation), [`StravaStore`] (so the loop
    /// can drain the Strava event queue), and [`Txn`] (so the loop can
    /// close the transaction).
    type Conn: StravaStore + Txn;

    /// Opens a new transaction on a fresh connection.
    async fn begin(&self) -> TbResult<Self::Conn>;
}

/// The transaction lifecycle of a connection, named for the executor loop
/// (ADR-0005, executable spec #446 §4.1).
///
/// While the transaction is open, a connection is a complete [`Store`];
/// `commit` and `rollback` close it. Both consume the connection, matching
/// `SqlxConn`'s inherent methods (the Postgres adapter returns the connection
/// to the pool).
#[async_trait::async_trait]
pub trait Txn: Store {
    /// Commits the transaction, consuming the connection.
    async fn commit(self) -> TbResult<()>;

    /// Rolls the transaction back, consuming the connection.
    async fn rollback(self) -> TbResult<()>;
}

/// A [`TxnSource`] wrapper that detaches `begin` from the caller's task:
/// the inner source's `begin` runs to completion in a spawned task, and the
/// caller awaits its result on a oneshot. A caller that cancels — a dropped
/// future (the executor's `tokio::select!` dropping the probe branch when a
/// request arrives, an axum handler dropped on client disconnect) — does not
/// cancel the wire-level `BEGIN`: the spawned task reads the `BEGIN`
/// response (so the client's transaction depth is incremented correctly) and
/// then drops the transaction it got — a clean lazy rollback that the pool
/// flushes on release. A plain `pool.begin()` cancelled between the `BEGIN`
/// being sent and its response being read would instead return the
/// connection to the pool inside a server-side transaction the client does
/// not know about (sqlx increments the depth only after the response, and
/// its drop guard is a no-op at depth 0) — the next `begin` on that
/// connection would issue a second, noticed `BEGIN` and its `COMMIT` would
/// commit the stale work with the new work (regression test
/// `request_burst_does_not_fire_transaction_notice` in
/// `tests/executor_seam.rs`).
///
/// This is an executor-level concern (managing the pool's lifecycle and
/// cancellation safety), not a sqlx-wrapper concern: the spawn lives here,
/// in `tb_exec`, where the executor owns the pool — `tb_sqlx` stays a thin
/// pool/conn wrapper.
#[derive(Clone)]
pub struct DetachedBegin<S: TxnSource>(S);

impl<S: TxnSource> DetachedBegin<S> {
    /// Wrap a transaction source with the cancellation-safe `begin`.
    pub fn new(source: S) -> Self {
        Self(source)
    }
}

#[async_trait::async_trait]
impl<S> TxnSource for DetachedBegin<S>
where
    S: TxnSource + Clone + Send + Sync + 'static,
{
    type Conn = S::Conn;

    async fn begin(&self) -> TbResult<S::Conn> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let source = self.0.clone();
        tokio::spawn(async move {
            let result = source.begin().await;
            let _ = tx.send(result);
        });
        rx.await
            .map_err(|err| Error::AnyFailure(anyhow::anyhow!("the begin's task ended: {err}")))?
    }
}
