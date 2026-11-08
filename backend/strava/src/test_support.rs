//! Test support for the Strava crate.
//!
//! - `test-support` feature → exposes [`seam`] to consumer crates (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → additionally exposes [`MockStravaDataStore`] (the `automock` witness, `tb_strava`-only).

pub mod seam;

#[cfg(test)]
pub use crate::traits::MockStravaDataStore;
pub use seam::*;
