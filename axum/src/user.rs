//! This module contains the implementation of user-related routes and handlers for the Axum web framework.
//!
//! The routes in this module are used to retrieve user information, summaries, and lists of users.
//! The handlers in this module interact with the database and Strava API to retrieve and process user data.
//!
//! This module also defines the `RUser` struct, which represents a user in the system and is used throughout the module.
//! Additionally, it defines the `AxumAdmin` struct, which is used as a marker type for routes that require admin privileges.

use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use tb_domain::{Person, Summary};
use tb_strava::StravaUser;

use crate::{appstate::AppState, ApiResult, AxumAdmin, DbPool, RequestUser};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/all", get(userlist))
}

async fn getuser(user: RequestUser, State(pool): State<DbPool>) -> ApiResult<tb_domain::User> {
    let mut conn = pool.get().await?;
    Ok(user.get_id().read(&mut conn).await.map(Json)?)
}

async fn summary(mut user: RequestUser, State(pool): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = pool.get().await?;
    StravaUser::update_user(&mut user, &mut conn).await?;
    StravaUser::process(&mut user, &mut conn).await?;
    Ok(user.get_id().get_summary(&mut conn).await.map(Json)?)
}

async fn userlist(
    _u: AxumAdmin,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<tb_strava::StravaStat>> {
    let mut conn = pool.get().await?;
    Ok(tb_strava::get_all_stats(&mut conn).await.map(Json)?)
}

pub(crate) async fn revoke_user(
    admin: AxumAdmin,
    Path(tbid): Path<i32>,
    State(pool): State<DbPool>,
) -> ApiResult<()> {
    let mut conn = pool.get().await?;
    let conn = &mut conn;
    let mut user = RequestUser::create_from_id(admin, tbid.into(), conn).await?;
    Ok(tb_strava::user_disable(&mut user, conn).await.map(Json)?)
}
