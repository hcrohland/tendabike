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

use crate::{ApiResult, AxumAdmin, DbPool, RequestSession, appstate::AppState};
use tb_domain::{Session, ShopId, Store, Summary};
use tb_strava::StravaUser;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/all", get(userlist))
        .route("/export", get(export))
}

async fn getuser(user: RequestSession, State(pool): State<DbPool>) -> ApiResult<tb_domain::User> {
    let mut store = pool.begin().await?;
    Ok(user.user_id().read(&mut store).await.map(Json)?)
}

#[derive(serde::Deserialize)]
struct ShopQuery {
    shop: Option<ShopId>,
}
async fn summary(
    mut session: RequestSession,
    State(pool): State<DbPool>,
    Query(ShopQuery { shop }): Query<ShopQuery>,
) -> ApiResult<Summary> {
    let mut store = pool.begin().await?;
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
    pub shops: Vec<tb_domain::ShopWithOwner>,
}

async fn export(user: RequestSession, State(pool): State<DbPool>) -> ApiResult<Export> {
    let mut store = pool.begin().await?;
    let user_id = user.user_id();
    let Summary {
        activities,
        parts,
        attachments,
        usages,
        services,
        plans,
        shops,
    } = user_id.get_summary(None, &mut store).await?;
    let user = user_id.read(&mut store).await?;
    Ok(Json(Export {
        user,
        activities,
        parts,
        attachments,
        usages,
        services,
        plans,
        shops,
    }))
}

async fn userlist(
    _u: AxumAdmin,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<tb_strava::StravaStat>> {
    let mut store = pool.begin().await?;
    Ok(tb_strava::get_all_stats(&mut store).await.map(Json)?)
}
