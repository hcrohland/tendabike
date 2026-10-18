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

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, error::AppError};
use tb_domain::{ApiRead, Error, ReadOutcome, Session, ShopId, Summary};
use tb_exec::TxnSource;
use tb_strava::StravaSession;

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>> {
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/stream", get(crate::stream::endpoint))
        .route("/all", get(userlist))
        .route("/export", get(export))
}

async fn getuser<S>(
    user: RequestSession,
    State(state): State<AppState<S>>,
) -> ApiResult<tb_domain::User>
where
    S: TxnSource + Clone + 'static,
{
    // The unified user read (spec #558 §3.5): the route projects the bare
    // `User` from the `UserState` composite.
    match state
        .registry
        .read(&state.source, user.tb_id(), ApiRead::User { shop: None })
        .await?
    {
        ReadOutcome::User(state) => Ok(Json(state.user)),
        // This route sent the `User` read; any other outcome is a bug, not
        // a domain outcome.
        _ => Err(AppError::TbError(Error::AnyFailure(anyhow::anyhow!(
            "the user read did not carry its user state"
        )))),
    }
}

#[derive(serde::Deserialize)]
struct ShopQuery {
    shop: Option<ShopId>,
}
async fn summary<S>(
    mut session: RequestSession,
    State(state): State<AppState<S>>,
    Query(ShopQuery { shop }): Query<ShopQuery>,
) -> ApiResult<Summary>
where
    S: TxnSource + Clone + 'static,
{
    // `set_shop`'s tower-session persistence stays in axum (spec #558
    // §3.5 — session plumbing, not domain state).
    session.set_shop(shop)?;
    // The unified user read (spec #558 §3.5): the route projects the
    // `Summary` from the `UserState` composite. The hydration write
    // (spec #558 §4) is the executor's own message, FIFO-ordered before
    // this read.
    match state
        .registry
        .read(&state.source, session.tb_id(), ApiRead::User { shop })
        .await?
    {
        ReadOutcome::User(state) => Ok(Json(state.summary)),
        // This route sent the `User` read; any other outcome is a bug, not
        // a domain outcome.
        _ => Err(AppError::TbError(Error::AnyFailure(anyhow::anyhow!(
            "the user read did not carry its user state"
        )))),
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

async fn export<S>(user: RequestSession, State(state): State<AppState<S>>) -> ApiResult<Export>
where
    S: TxnSource + Clone + 'static,
{
    // The unified user read (spec #558 §3.5): the export is a projection
    // of the `UserState` composite (the bare `User` plus the `Summary`),
    // staying non-hydrating (spec #558 §3.4/§4).
    match state
        .registry
        .read(&state.source, user.tb_id(), ApiRead::User { shop: None })
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
        // This route sent the `User` read; any other outcome is a bug, not
        // a domain outcome.
        _ => Err(AppError::TbError(Error::AnyFailure(anyhow::anyhow!(
            "the user read did not carry its user state"
        )))),
    }
}

async fn userlist<S>(
    user: RequestSession,
    _admin: AxumAdmin,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<tb_strava::StravaStat>>
where
    S: TxnSource + Clone + 'static,
{
    // The admin read rides the admin user's own channel (spec #558 §3.3):
    // the `AxumAdmin` gate is unchanged; the session's identity is no
    // longer discarded.
    Ok(Json(
        state
            .registry
            .all_stats(&state.source, user.tb_id())
            .await?,
    ))
}
