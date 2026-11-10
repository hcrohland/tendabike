//! The intervals.icu webhook endpoint (spec #764): `POST /icu/callback`.
//!
//! intervals.icu POSTs a JSON envelope with a shared `secret` and an array of
//! `events`. The route verifies the `secret` (a constant-time compare against
//! `ICU_WEBHOOK_SECRET`), then — for each `ACTIVITY_ANALYZED` event (the only
//! event type subscribed) — maps the `athlete_id` to a TendaBike user and
//! queues an `icu_events` row, firing the wake signal. It answers `200`
//! immediately (the processing is async via the executor); a bad secret is a
//! `401` and a failed ingest a `500` (intervals.icu's own retry policy
//! applies).

use axum::{Json, extract::State};
use log::trace;
use serde_derive::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use tb_domain::{Error, TbResult};
use tb_provider::icu::{IcuEvent, IcuId};

use crate::{ApiResult, appstate::AppState};

/// The intervals.icu webhook envelope (the `POST /icu/callback` body).
#[derive(Debug, Deserialize)]
pub(crate) struct IcuWebhook {
    /// The app's shared webhook secret (verified against
    /// `ICU_WEBHOOK_SECRET`).
    secret: String,
    /// The events in the delivery (an intervals.icu app has one webhook URL
    /// for every athlete who authorized it, so a delivery can carry several).
    events: Vec<IcuWebhookEvent>,
}

/// One event in an intervals.icu webhook delivery.
#[derive(Debug, Deserialize)]
pub(crate) struct IcuWebhookEvent {
    /// The icu athlete the event belongs to.
    athlete_id: String,
    /// The event type (e.g. `ACTIVITY_ANALYZED`); only `ACTIVITY_ANALYZED` is
    /// subscribed.
    #[serde(rename = "type")]
    type_: String,
    /// The event's RFC 3339 timestamp.
    timestamp: String,
    /// The activity the event refers to (its id is the `icu_events` key).
    activity: IcuWebhookActivity,
}

/// The activity an intervals.icu activity event refers to.
#[derive(Debug, Deserialize)]
pub(crate) struct IcuWebhookActivity {
    id: String,
}

/// The `ACTIVITY_ANALYZED` event type: the only event the webhook subscribes
/// to (spec #764).
const ACTIVITY_ANALYZED: &str = "ACTIVITY_ANALYZED";

/// The shared webhook secret (the app's intervals.icu webhook secret, the
/// `ICU_WEBHOOK_SECRET` env var).
fn webhook_secret() -> String {
    std::env::var("ICU_WEBHOOK_SECRET").unwrap_or_default()
}

/// A constant-time byte equality (the webhook secret check, spec #764): avoids
/// a timing side-channel on the shared secret. A length mismatch is not secret
/// (the secret's length is fixed per app), so it short-circuits.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Verify the webhook's shared secret (spec #764): a constant-time compare
/// against `ICU_WEBHOOK_SECRET`. A mismatch (or an unset secret) is a `401`.
fn verify_secret(secret: &str) -> TbResult<()> {
    let expected = webhook_secret();
    if !constant_time_eq(secret.as_bytes(), expected.as_bytes()) {
        return Err(Error::NotAuth("invalid icu webhook secret".to_string()));
    }
    Ok(())
}

/// Parse an intervals.icu webhook timestamp (RFC 3339) to a unix second.
fn parse_timestamp(timestamp: &str) -> TbResult<i64> {
    OffsetDateTime::parse(timestamp, &Rfc3339)
        .map(|dt| dt.unix_timestamp())
        .map_err(|_| Error::BadRequest(format!("invalid icu webhook timestamp {timestamp}")))
}

/// `POST /icu/callback` (the webhook, spec #764): verify the shared `secret`,
/// then — for each `ACTIVITY_ANALYZED` event — map the `athlete_id` to a
/// TendaBike user and queue an `icu_events` row (the `IcuEvent::accept`
/// direct op, which also fires the wake). Answers `200` immediately (the
/// processing is async via the executor); a bad secret is a `401`, a failed
/// ingest a `500` (intervals.icu's own retry policy applies).
pub(crate) async fn create_event(
    State(state): State<AppState>,
    Json(webhook): axum::extract::Json<IcuWebhook>,
) -> ApiResult<()> {
    trace!("Received icu webhook with {} events", webhook.events.len());
    verify_secret(&webhook.secret)?;
    for event in &webhook.events {
        if event.type_ != ACTIVITY_ANALYZED {
            continue; // only ACTIVITY_ANALYZED is subscribed (spec #764)
        }
        let icu_event = IcuEvent::new(
            IcuId::from(event.athlete_id.as_str()),
            event.activity.id.clone(),
            parse_timestamp(&event.timestamp)?,
        );
        state.icu.icu_accept_event(icu_event).await?;
    }
    Ok(Json(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_equal_bytes() {
        assert!(constant_time_eq(b"secret", b"secret"));
    }

    #[test]
    fn constant_time_eq_rejects_differing_bytes() {
        assert!(!constant_time_eq(b"secret", b"secrex"));
    }

    #[test]
    fn constant_time_eq_rejects_differing_lengths() {
        assert!(!constant_time_eq(b"secret", b"secrets"));
    }

    #[test]
    fn parse_timestamp_reads_rfc3339() {
        assert_eq!(
            parse_timestamp("2024-12-06T06:40:47.011+00:00").unwrap(),
            1733467247
        );
    }

    #[test]
    fn parse_timestamp_rejects_garbage() {
        assert!(matches!(
            parse_timestamp("not-a-timestamp"),
            Err(Error::BadRequest(_))
        ));
    }
}
