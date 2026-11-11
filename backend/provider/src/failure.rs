//! The event-failure policy (spec #446 §4.6), owned by the provider kit:
//! the kit classifies a failed provider call into a decision — a pure
//! function, testable without a store — and each provider module applies
//! the decision to its own tables (and logs).

use tb_domain::Error;

/// The failure semantics of a failed provider call (spec #446 §4.6) —
/// drop on permanent failure, no dead-letter table:
///
/// - a rate limit (`TryAgain`) → keep the event queued: it is retried
///   after the backoff.
/// - dead auth (`NotAuth`, a 401 after a token refresh) → disable the
///   user's integration, which drops all their queued events: the data
///   freezes (stale but consistent), and no event-by-event 401s follow.
/// - a DB failure → propagate: the caller rolls back (the event stays
///   queued) and applies its in-memory backoff.
/// - anything else is permanent (a 404 means the activity is gone; a parse
///   error will not heal): drop the event, log it, and move on — the
///   admin sync endpoint re-covers a gap if the user cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDecision {
    /// The event stays queued (a rate limit): it is retried after the backoff.
    KeepQueued,
    /// The user's integration is dead (a 401 after a token refresh): disable
    /// it — which drops all their queued events.
    DisableUser,
    /// The event is permanently bad (a 404, a parse error): drop it, log it,
    /// and move on.
    DropEvent,
    /// A DB failure: propagate it — the caller rolls back (the event stays
    /// queued) and applies its in-memory backoff.
    Propagate,
}

/// Classifies a failed provider call into the queue's failure decision
/// (spec #446 §4.6).
///
/// Pure: no store access, no side effects. Each provider module maps the
/// decision onto its own tables and logs.
pub fn classify(err: &Error) -> FailureDecision {
    match err {
        Error::TryAgain(_) => FailureDecision::KeepQueued,
        Error::NotAuth(_) => FailureDecision::DisableUser,
        Error::DatabaseFailure(_) => FailureDecision::Propagate,
        _ => FailureDecision::DropEvent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_again_keeps_the_event_queued() {
        assert_eq!(
            classify(&Error::TryAgain("rate limit")),
            FailureDecision::KeepQueued
        );
    }

    #[test]
    fn not_auth_disables_the_user() {
        assert_eq!(
            classify(&Error::NotAuth("token dead".to_string())),
            FailureDecision::DisableUser
        );
    }

    #[test]
    fn database_failure_propagates() {
        assert_eq!(
            classify(&Error::DatabaseFailure(anyhow::anyhow!("db down"))),
            FailureDecision::Propagate
        );
    }

    #[test]
    fn permanent_errors_drop_the_event() {
        for err in [
            Error::NotFound("gone".to_string()),
            Error::BadRequest("bad".to_string()),
            Error::AnyFailure(anyhow::anyhow!("parse")),
        ] {
            assert_eq!(classify(&err), FailureDecision::DropEvent);
        }
    }
}
