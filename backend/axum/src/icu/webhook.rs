//! The intervals.icu webhook endpoint (spec #764): `POST /icu/callback`.
//!
//! The web shell keeps only the protocol (ADR-0014): it forwards the raw
//! body opaquely to the provider service's `accept_event` op — the envelope
//! parse, the shared `secret` verification (a constant-time compare against
//! `ICU_WEBHOOK_SECRET`), the event filtering (only `ACTIVITY_ANALYZED` is
//! subscribed), the timestamp parse, and the queue insert are the provider
//! layer's work. It answers `200` immediately (the processing is async via
//! the executor); a refused delivery (a bad secret) is a `401` and a failed
//! ingest a `500` (intervals.icu's own retry policy applies).

use axum::{Json, extract::State};

use crate::{ApiResult, appstate::AppState};
use tb_provider::Provider;

/// `POST /icu/callback` (the webhook, spec #764): forward the raw body to
/// the provider service's `accept_event` op (the provider layer verifies the
/// shared `secret`, filters the events, and queues the `icu_events` rows,
/// firing the wake). Answers `200` immediately (the processing is async via
/// the executor); a refused delivery is a `401`, a failed ingest a `500`
/// (intervals.icu's own retry policy applies).
pub(crate) async fn create_event(State(state): State<AppState>, body: String) -> ApiResult<()> {
    state.provider.accept_event(Provider::Icu, &body).await?;
    Ok(Json(()))
}
