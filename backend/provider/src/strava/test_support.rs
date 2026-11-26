//! Test support for the Strava driver.
//!
//! - `test-support` feature → exposes [`seam`] to consumer crates (`tb_exec`,
//!   `tb_sqlx`).
//! - `#[cfg(test)]` → exposes the [`seam`] to the Strava module's own tests.
//!
//! The store double is the single [`MockStore`](crate::test_support::MockStore)
//! over [`StravaSpec`](super::StravaSpec) (spec #1029) — the per-provider
//! data-store mock is gone.

pub mod seam;

pub use seam::*;
