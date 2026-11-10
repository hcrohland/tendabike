//! Test support for the Strava driver.
//!
//! - `test-support` feature → exposes [`seam`] and [`MockStravaDataStore`] to consumer crates (`tb_exec`, `tb_sqlx`).
//! - `#[cfg(test)]` → exposes [`MockStravaDataStore`] (the `automock` witness) to the Strava module's own tests.

pub mod seam;

#[cfg(any(test, feature = "test-support"))]
pub use super::traits::MockStravaDataStore;
pub use seam::*;
