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
//! a concrete adapter — the `StravaStore` bound sits on the loop, so this
//! seam stays lifecycle-only.

use tb_domain::{Store, TbResult};

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
    /// A connection with an open transaction: every [`Store`] sub-trait (so
    /// it can run the domain operation) and [`Txn`] (so the loop can close
    /// the transaction).
    type Conn: Store + Txn;

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
