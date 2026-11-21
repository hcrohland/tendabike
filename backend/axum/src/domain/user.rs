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
    http::StatusCode,
    routing::get,
};
use serde::Serialize;

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState, domain::result_mismatch};
use tb_domain::{
    ApiRead, ApiWrite, Error, GearItem, ReadOutcome, Session, Summary, User, WriteOutcome,
};

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/ping", get(ping))
        .route("/stream", get(crate::stream::endpoint))
        .route("/all", get(userlist))
        .route("/export", get(export))
}

/// The wire shape of `GET /api/user`: the user record (with the provider
/// added — the domain stores no provider, ADR-0014) plus the `Summary`.
#[derive(Serialize)]
struct UserResponse {
    user: UserWithProvider,
    summary: Summary,
}

/// The response's user shape: the domain `User`'s fields plus the `provider`
/// (the provider layer's identity, ADR-0014). The client's `user.provider`
/// wire field is unchanged.
#[derive(Serialize)]
struct UserWithProvider {
    #[serde(flatten)]
    user: User,
    provider: String,
}

/// `GET /api/user` — the one-call initial state (spec #572): returns the user
/// record (with the provider added) plus the `Summary`. Replaces the old
/// bare-user and summary-only routes. A pure read (spec #829): it makes no
/// outbound provider call for gear — the gear import happens at onboarding.
/// The user's provider is the provider layer's identity lookup (ADR-0014):
/// the domain stores no provider, so the web layer derives it and adds it to
/// the response's user shape. A `NotFound` means the user was deleted: the
/// session is stale, so we reply 401.
async fn getuser(
    session: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<UserResponse> {
    // The user's provider (ADR-0014): the domain stores no provider, so the
    // web layer derives it from the provider-internal user data and adds it
    // to the response's user shape. A `NotFound` means the user was deleted:
    // the session is stale, so we reply 401.
    let provider = match state.provider.user_provider(session.user_id()).await {
        Ok(p) => p,
        Err(Error::NotFound(_)) => return Err(Error::NotAuth("session expired".to_string()).into()),
        Err(err) => return Err(err.into()),
    };
    // The unified user read (spec #558 §3.5): a single FIFO-ordered message
    // on the user's executor — the pure-FIFO snapshot (spec #558 §3.7). A
    // pure read (spec #829): no outbound provider call for gear.
    match state.api.read(Box::new(session), ApiRead::User).await {
        Ok(ReadOutcome::User(user_state)) => {
            let user_state = *user_state;
            Ok(Json(UserResponse {
                user: UserWithProvider {
                    user: user_state.user,
                    provider: provider.wire_value().to_string(),
                },
                summary: user_state.summary,
            }))
        }
        Ok(_) => Err(result_mismatch("user", "user state")),
        Err(Error::NotFound(_)) => Err(Error::NotAuth("session expired".to_string()).into()),
        Err(err) => Err(err.into()),
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

/// The onboarding gear import response (spec #829, the first step): the list
/// of gear items the provider import discovered (name + type), so the
/// onboarding dialog can present them.
#[derive(Serialize, Debug)]
pub(crate) struct OnboardingGearResponse {
    pub gear: Vec<GearItem>,
}

/// The onboarding sync's query (the `time`, 0 = "now").
#[derive(serde::Deserialize)]
pub(crate) struct InitialSyncQuery {
    #[serde(default)]
    pub(crate) time: i64,
}

/// `POST /api/onboarding/gear` (spec #829, the first step): import the
/// user's gear from the provider and return the discovered gear items. No
/// activity sync is queued — the user decides on that next (the second step,
/// [`trigger_initial_sync`]). The domain's status guard runs on the user's
/// executor; the gear import is the provider's work in the same transaction.
pub(crate) async fn trigger_gear_import(
    user: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<OnboardingGearResponse> {
    let outcome = state
        .api
        .write(Box::new(user), ApiWrite::UserOnboardingGear)
        .await?;
    let gear = match outcome {
        WriteOutcome::OnboardingGear { gear, .. } => gear,
        _ => return Err(result_mismatch("onboarding gear import", "gear list")),
    };
    Ok(Json(OnboardingGearResponse { gear }))
}

/// `POST /api/onboarding/sync` (spec #829, the second step): queue the
/// activity sync and return the updated user. No gear import — that was the
/// first step ([`trigger_gear_import`]). The domain's status guard and
/// completion update run on the user's executor in one loop-owned transaction
/// alongside the provider-side `queue_sync`. The 200 body reads the `User`
/// record back (the light record-only read — no `Summary` computation needed).
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

/// `POST /api/onboarding/postpone` (spec #974): postpone the initial
/// activity sync and return the updated user. Provider-agnostic — mounted
/// once at the neutral path (the user's provider is derived from their
/// identity in-transaction, ADR-0014). The domain's status guard and
/// postponement run on the user's executor (spec §6.2); the 200 body
/// reads the `User` record back (the light record-only read — no `Summary`
/// computation needed).
pub(crate) async fn postpone_initial_sync(
    user: RequestSession,
    State(state): State<AppState>,
) -> ApiResult<tb_domain::User> {
    state
        .api
        .write(Box::new(user.clone()), ApiWrite::UserOnboardingPostpone)
        .await?;
    match state.api.read(Box::new(user), ApiRead::UserRecord).await? {
        ReadOutcome::UserRecord(user) => Ok(Json(user)),
        _ => Err(result_mismatch("user", "user record")),
    }
}

async fn export(user: RequestSession, State(state): State<AppState>) -> ApiResult<Export> {
    // The unified user read (spec #558 §3.5): the export is a projection
    // of the `UserState` composite (the bare `User` plus the `Summary`),
    // staying non-hydrating (spec #558 §3.4/§4). A `NotFound` means the
    // user was deleted: the session is stale, so we reply 401.
    match state.api.read(Box::new(user), ApiRead::User).await {
        Ok(ReadOutcome::User(state)) => {
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
        Ok(_) => Err(result_mismatch("user", "user state")),
        Err(Error::NotFound(_)) => Err(Error::NotAuth("session expired".to_string()).into()),
        Err(err) => Err(err.into()),
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
