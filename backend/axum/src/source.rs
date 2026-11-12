//! Provider-neutral redirect routes.
//!
//! `/source/activities/{id}` redirects to the activity's provider URL. The
//! id is provider-prefixed (`strava:12345` / `icu:i55751783`), and the
//! service API's `activity_url` op routes on that prefix.

use axum::{
    Router,
    extract::{Path, State},
    response::Redirect,
    routing::get,
};
use tb_domain::ActivityId;

use crate::{RequestSession, appstate::AppState, error::AppError};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/activities/{id}", get(redirect_act))
}

async fn redirect_act(
    user: RequestSession,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    // The redirect direct op (spec #558 §5.2): the service API's
    // `activity_url` op routes on the id's provider prefix and checks
    // ownership; the handler maps `Err` → a permanent redirect to "/".
    let uri = state
        .strava
        .activity_url(Box::new(user), ActivityId::from(id))
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}
