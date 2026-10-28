//! Test fixtures for the DB-free domain test suites.
//!
//! Shared, store-agnostic helpers: a fixed test user and the [`MockSession`]
//! built from it, and a fixed sample purchase date. The suites have no wall
//! clock, so every fixture time is a fixed value.

use super::{MockSession, mock_session};
use crate::{OffsetDateTime, UserId};

/// Returns a test UserId (ID = 1).
pub fn test_user() -> UserId {
    UserId::from(1)
}

/// Returns a [`MockSession`] initialized with test_user().
pub fn test_session() -> MockSession {
    mock_session(test_user())
}

/// Returns a sample purchase date (fixed for deterministic tests).
pub fn sample_purchase_date() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1700000000).unwrap()
}
