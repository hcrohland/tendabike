//! This module contains the implementation of user-related routes and handlers for the Axum web framework.
//!
//! The routes in this module are used to retrieve user information, summaries, and lists of users.
//! The handlers in this module interact with the database and Strava API to retrieve and process user data.
//!
//! This module also defines the `RUser` struct, which represents a user in the system and is used throughout the module.
//! Additionally, it defines the `AxumAdmin` struct, which is used as a marker type for routes that require admin privileges.

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Serialize;

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, domain::read_mismatch};
use tb_domain::{ApiRead, ReadOutcome, Session, ShopId, Summary};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/stream", get(crate::stream::endpoint))
        .route("/all", get(userlist))
        .route("/export", get(export))
}

async fn getuser(
    user: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<tb_domain::User> {
    // The unified user read (spec #558 §3.5): the route projects the bare
    // `User` from the `UserState` composite.
    match state
        .api
        .read(user.user_id(), ApiRead::User { shop: None })
        .await?
    {
        ReadOutcome::User(state) => Ok(Json(state.user)),
        _ => Err(read_mismatch("user", "user state")),
    }
}

#[derive(serde::Deserialize)]
struct ShopQuery {
    shop: Option<ShopId>,
}
async fn summary(
    mut session: RequestSession,
    State(state): State<AppState>,
    Query(ShopQuery { shop }): Query<ShopQuery>,
) -> ApiResult<Summary> {
    // `set_shop`'s tower-session persistence stays in axum (spec #558
    // §3.5 — session plumbing, not domain state).
    session.set_shop(shop)?;
    // The hydration write (spec #558 §4), then the unified user read (spec
    // #558 §3.5): two FIFO-ordered messages on the user's executor — the
    // loop runs `update_gear` in its own transaction, and the read's
    // pure-FIFO snapshot (spec #558 §3.7) sees the committed hydration, so
    // the response body carries the new parts exactly as today. The route
    // projects the `Summary` from the `UserState` composite.
    state.api.hydrate_gear(session.user_id()).await?;
    match state
        .api
        .read(session.user_id(), ApiRead::User { shop })
        .await?
    {
        ReadOutcome::User(state) => Ok(Json(state.summary)),
        _ => Err(read_mismatch("user", "user state")),
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct Export {
    pub user: tb_domain::User,
    pub parts: Vec<tb_domain::Part>,
    pub attachments: Vec<tb_domain::AttachmentDetail>,
    pub services: Vec<tb_domain::Service>,
    pub plans: Vec<tb_domain::ServicePlan>,
    pub usages: Vec<tb_domain::Usage>,
    pub activities: Vec<tb_domain::Activity>,
    pub shops: Vec<tb_domain::Shop>,
}

async fn export(user: RequestSession, State(state): State<AppState>) -> ApiResult<Export> {
    // The unified user read (spec #558 §3.5): the export is a projection
    // of the `UserState` composite (the bare `User` plus the `Summary`),
    // staying non-hydrating (spec #558 §3.4/§4).
    match state
        .api
        .read(user.user_id(), ApiRead::User { shop: None })
        .await?
    {
        ReadOutcome::User(state) => {
            let summary = state.summary;
            Ok(Json(Export {
                user: state.user,
                activities: summary.activities.live(),
                parts: summary.parts.live(),
                attachments: summary.attachments.live(),
                usages: summary.usages.live(),
                services: summary.services.live(),
                plans: summary.plans.live(),
                shops: summary.shops.live(),
            }))
        }
        _ => Err(read_mismatch("user", "user state")),
    }
}

async fn userlist(
    user: RequestSession,
    _admin: AxumAdmin,
    State(state): State<AppState>,
) -> ApiResult<Vec<tb_strava::StravaStat>> {
    // The admin read rides the admin user's own channel (spec #558 §3.3):
    // the `AxumAdmin` gate is unchanged; the session's identity is no
    // longer discarded.
    Ok(Json(state.api.all_stats(user.user_id()).await?))
}
