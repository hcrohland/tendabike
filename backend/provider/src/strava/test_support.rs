//! Test support for the Strava driver.
//!
//! - `test-support` feature → exposes [`seam`] to consumer crates (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → additionally exposes [`MockStravaDataStore`] (the `automock` witness, the Strava module's own tests only).

pub mod seam;

#[cfg(any(test, feature = "test-support"))]
pub use super::traits::MockStravaDataStore;
pub use seam::*;
