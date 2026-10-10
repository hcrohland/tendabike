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

use crate::{ApiResult, AxumAdmin, RequestSession, appstate::AppState};
use tb_domain::{Session, ShopId, Summary};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaStore;
use tb_strava::StravaUser;

pub(super) fn router<S: TxnSource + Clone + 'static>() -> Router<AppState<S>>
where
    S::Conn: StravaStore,
{
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
    let mut store = state.source.begin().await?;
    Ok(user.user_id().read(&mut store).await.map(Json)?)
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
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    session.set_shop(shop)?;
    StravaUser::update_gear(&mut session, &mut store).await?;
    let res = session
        .user_id()
        .get_summary(session.shop(), &mut store)
        .await
        .map(Json)?;
    store.commit().await?;
    Ok(res)
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
    let mut store = state.source.begin().await?;
    let user_id = user.user_id();
    let summary = user_id.get_summary(None, &mut store).await?;
    let user = user_id.read(&mut store).await?;
    Ok(Json(Export {
        user,
        activities: summary.activities.live(),
        parts: summary.parts.live(),
        attachments: summary.attachments.live(),
        usages: summary.usages.live(),
        services: summary.services.live(),
        plans: summary.plans.live(),
        shops: summary.shops.live(),
    }))
}

async fn userlist<S>(
    _u: AxumAdmin,
    State(state): State<AppState<S>>,
) -> ApiResult<Vec<tb_strava::StravaStat>>
where
    S: TxnSource + Clone + 'static,
    S::Conn: StravaStore,
{
    let mut store = state.source.begin().await?;
    Ok(tb_strava::get_all_stats(&mut store).await.map(Json)?)
}
