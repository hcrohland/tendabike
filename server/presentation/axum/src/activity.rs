use axum::{
    extract::{Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use http::StatusCode;
use kernel::domain::{Activity, ActivityId, NewActivity, PartId, PartTypeId, Summary};

use crate::{
    error::{ApiResult, AppError},
    user::{AxumAdmin, RUser},
    AppDbConn,
};

async fn def_part_api(
    user: RUser,
    mut conn: AppDbConn,
    Json(gear_id): Json<PartId>,
) -> ApiResult<Summary> {
    Ok(Activity::set_default_part(gear_id, &user, &mut conn).map(Json)?)
}

async fn rescan(_u: AxumAdmin, mut conn: AppDbConn) -> Result<(), AppError> {
    Activity::rescan_all(&mut conn)?;
    Ok(())
}

/// web interface to read an activity
async fn act_get(user: RUser, mut conn: AppDbConn, Path(id): Path<i32>) -> ApiResult<Activity> {
    Ok(ActivityId::new(id).read(&user, &mut conn).map(Json)?)
}

/// web interface to create an activity
async fn act_post(
    user: RUser,
    mut conn: AppDbConn,
    Json(activity): Json<NewActivity>,
) -> Result<(StatusCode, Json<Summary>), AppError> {
    let assembly = Activity::create(&activity, &user, &mut conn)?;

    Ok((StatusCode::CREATED, Json(assembly)))
}

/// web interface to change an activity
async fn act_put(
    Path(id): Path<i32>,
    user: RUser,
    mut conn: AppDbConn,
    activity: Json<NewActivity>,
) -> ApiResult<Summary> {
    Ok(ActivityId::new(id)
        .update(&activity, &user, &mut conn)
        .map(Json)?)
}

/// web interface to delete an activity
async fn act_delete(Path(id): Path<i32>, user: RUser, mut conn: AppDbConn) -> ApiResult<Summary> {
    Ok(ActivityId::new(id).delete(&user, &mut conn).map(Json)?)
}

async fn descend(Query(tz): Query<String>, user: RUser, mut conn: AppDbConn, data: String) -> ApiResult<(Summary, Vec<String>, Vec<String>)> {
    let res = tokio::task::spawn_blocking(move || Activity::csv2descend(data.as_bytes(), tz, &user, &mut conn)).await??;
    Ok(Json(res))
}

async fn mycats(user: RUser, mut conn: AppDbConn) -> ApiResult<Vec<PartTypeId>> {
    Ok(Activity::categories(&user, &mut conn).map(Json)?)
}

pub(crate) fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/categories", get(mycats))
        .route("/descend", post(descend))
        .route("/:id", delete(act_delete).get(act_get).put(act_put))
        .route("/", post(act_post))
        .route("/rescan", get(rescan))
        .route("/defaultgear", post(def_part_api))
        .with_state(state)
}
