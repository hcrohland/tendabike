use axum::{routing::post, Json, Router};
use kernel::domain::{Event, Summary};

use crate::{error::ApiResult, user::RUser, AppDbConn};

/// route for attach API
async fn attach_rt(
    user: RUser,
    mut conn: AppDbConn,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    Ok(event.attach(&user, &mut conn).map(Json)?)
}

/// route for detach API
async fn detach_rt(
    user: RUser,
    mut conn: AppDbConn,
    Json(event): Json<Event>,
) -> ApiResult<Summary> {
    Ok(event.detach(&user, &mut conn).map(Json)?)
}

pub(crate) fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/attach", post(attach_rt))
        .route("/detach", post(detach_rt))
        .with_state(state)
}
