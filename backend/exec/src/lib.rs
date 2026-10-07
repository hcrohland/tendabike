//! The per-user executor crate (ADR-0005, executable spec #446 §4): this ticket adds the transaction seam (the `Txn` and `TxnSource` traits); the executor loop itself lands in a later ticket.

mod txn;
pub use txn::*;

#[cfg(test)]
mod tests;
