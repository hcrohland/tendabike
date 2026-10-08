//! The per-user executor crate (ADR-0005, executable spec #446 §4).
//!
//! One long-lived task per user drives the domain: it consumes the user's
//! API-write channel (in-memory, one per user) and the `strava_events`
//! database queue, runs each message in one transaction through the
//! [`TxnSource`]/[`Txn`] seam, and pushes the resulting `Summary` to the
//! user's SSE streams.
//!
//! - [`txn`] — the `Txn` and `TxnSource` traits: the transaction lifecycle
//!   named for the loop. The loop is generic over the source and never names
//!   a concrete adapter.
//! - [`message`] — the [`Message`] enum and the in-memory plumbing (the
//!   per-user API-write channel, the oneshot reply, the `Summary` frame sink).
//! - [`executor`] — [`run`], the loop itself: a biased select, one
//!   transaction per message, and the reclaim-on-idle policy.
//!
//! The loop computes no domain state of its own: it drives the existing
//! `ApiWrite` dispatch and `tb_strava` event processing inside transactions
//! it owns. `MemStore` deliberately implements neither trait (a
//! `tb_domain` → `tb_exec` dependency would be a cycle); the seam's Postgres
//! half lives in `tb_sqlx`, and the transaction integration is verified
//! against real Postgres in `tb_sqlx`'s `tests/executor_seam.rs`.

mod executor;
mod message;
mod txn;

pub use executor::*;
pub use message::*;
pub use txn::*;

#[cfg(test)]
mod executor_tests;
#[cfg(test)]
mod tests;
