//! Test support for the Strava crate.
//!
//! - `test-support` feature → exposes [`seam`] to consumer crates (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → additionally exposes [`mock`] (the `mockall` witness, `tb_strava`-only).

#[cfg(test)]
pub mod mock;
pub mod seam;

#[cfg(test)]
pub use mock::*;
pub use seam::*;
