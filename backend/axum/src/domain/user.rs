//! This module contains the implementation of user-related routes and handlers for the Axum web framework.
//!
//! The routes in this module are used to retrieve user information, summaries, and lists of users.
//! The handlers in this module interact with the database and Strava API to retrieve and process user data.
//!
//! This module also defines the `RUser` struct, which represents a user in the system and is used throughout the module.
//! Additionally, it defines the `AxumAdmin` struct, which is used as a marker type for routes that require admin privileges.

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, domain::result_mismatch};
use tb_domain::{ApiRead, ReadOutcome, UserState};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/ping", get(ping))
        .route("/stream", get(crate::stream::endpoint))
        .route("/all", get(userlist))
        .route("/export", get(export))
}

/// `GET /api/user` — the one-call initial state (spec #572): returns the
/// full `UserState` (the user record plus the `Summary`). Replaces the old
/// bare-user and summary-only routes. A pure read (spec #829): it makes no
/// outbound provider call for gear — the gear import moved to onboarding.
async fn getuser(session: RequestSession, State(state): State<AppState>) -> ApiResult<UserState> {
    // The unified user read (spec #558 §3.5): a single FIFO-ordered message
    // on the user's executor — the pure-FIFO snapshot (spec #558 §3.7).
    match state.api.read(Box::new(session), ApiRead::User).await? {
        ReadOutcome::User(state) => Ok(Json(*state)),
        _ => Err(result_mismatch("user", "user state")),
    }
}

/// `GET /api/user/ping` — content-free liveness probe (spec #572): returns
/// `204` for an authenticated session, `401` otherwise. No body, no DB,
/// no executor work — the `RequestSession` extractor is the entire check.
async fn ping(_user: RequestSession) -> StatusCode {
    StatusCode::NO_CONTENT
}

#[derive(Clone, Serialize, Debug)]
struct Export {
    pub user: tb_domain::User,
    pub parts: Vec<tb_domain::Part>,
    pub attachments: Vec<tb_domain::AttachmentDetail>,
    pub services: Vec<tb_domain::Service>,
    pub plans: Vec<tb_domain::ServicePlan>,
    pub usages: Vec<tb_domain::Usage>,
    pub activities: Vec<tb_domain::Activity>,
    pub shops: Vec<tb_domain::Shop>,
}

/// The onboarding sync response (spec #829): the updated `User` plus the
/// list of gear items the provider import discovered (name + type), so the
/// onboarding dialog can present them. The gear parts themselves ride the
/// write's `Summary` frame; this is the dialog's response body.
#[derive(Serialize, Debug)]
pub(crate) struct OnboardingResponse {
    pub user: tb_domain::User,
    pub gear: Vec<tb_domain::GearItem>,
}

async fn export(user: RequestSession, State(state): State<AppState>) -> ApiResult<Export> {
    // The unified user read (spec #558 §3.5): the export is a projection
    // of the `UserState` composite (the bare `User` plus the `Summary`),
    // staying non-hydrating (spec #558 §3.4/§4).
    match state.api.read(Box::new(user), ApiRead::User).await? {
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
        _ => Err(result_mismatch("user", "user state")),
    }
}

async fn userlist(
    admin: AxumAdmin,
    State(state): State<AppState>,
) -> ApiResult<Vec<tb_domain::UserStat>> {
    // The admin read rides the admin user's own channel (spec #558 §3.3):
    // the `AxumAdmin` extractor carries the admin's acting session, which
    // the stats read runs against.
    Ok(Json(state.api.all_stats(Box::new(admin.session())).await?))
}
