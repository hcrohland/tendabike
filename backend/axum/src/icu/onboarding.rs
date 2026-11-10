//! The icu onboarding sync route (spec #766): `POST /icu/onboarding/sync`.
//!
//! Triggers the icu onboarding backfill — the gear import plus the date-window
//! activity walk, which queues the user's past activities as `icu_events`
//! rows. The executor's normal loop then imports them in the background
//! (non-blocking: this returns once the gear is imported and the activities
//! are queued). Returns the updated user.

use axum::{
    Json,
    extract::{Query, State},
};

use crate::{ApiResult, RequestSession, appstate::AppState, domain::result_mismatch};
use tb_domain::{ApiRead, ApiWrite, ReadOutcome};

#[derive(serde::Deserialize)]
pub(super) struct InitialSyncQuery {
    #[serde(default)]
    time: i64,
}

/// `POST /icu/onboarding/sync`: trigger the icu onboarding backfill (the gear
/// import + the activity walk). The domain's status guard and completion
/// update run on the user's executor in one loop-owned transaction alongside
/// the icu-side `insert_sync`. Returns the updated user.
pub(crate) async fn trigger_initial_sync(
    user: RequestSession,
    State(state): State<AppState>,
    Query(query): Query<InitialSyncQuery>,
) -> ApiResult<tb_domain::User> {
    state
        .api
        .write(
            Box::new(user.clone()),
            ApiWrite::UserOnboardingSync { time: query.time },
        )
        .await?;
    match state.api.read(Box::new(user), ApiRead::UserRecord).await? {
        ReadOutcome::UserRecord(user) => Ok(Json(user)),
        _ => Err(result_mismatch("user", "user record")),
    }
}
