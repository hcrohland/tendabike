use crate::{
    error::{ApiResult, AppError},
    user::RUser,
    AppDbConn,
};

use axum::{
    extract::Path,
    routing::{get, put},
    Json, Router,
};
use http::StatusCode;
use kernel::domain::{ChangePart, NewPart, Part, PartId};

pub(crate) fn router(state: crate::AppState) -> Router {
    Router::new()
        .route("/", put(put_part).post(post_part))
        .route("/:part", get(get_part))
        .with_state(state)
}

// #[get("/<part>")]
async fn get_part(Path(part): Path<i32>, user: RUser, mut conn: AppDbConn) -> ApiResult<Part> {
    Ok(PartId::new(part).part(&user, &mut conn).map(Json)?)
}

// #[post("/", data = "<newpart>")]
async fn post_part(
    user: RUser,
    mut conn: AppDbConn,
    Json(newpart): Json<NewPart>,
    // ) -> ApiResult<Part> {
) -> Result<(StatusCode, Json<Part>), AppError> {
    let part = newpart.clone().create(&user, &mut conn)?;
    // let url = rocket::uri!(get: i32::from(part.id));
    Ok((StatusCode::CREATED, Json(part)))
}

// #[put("/", data = "<part>")]
async fn put_part(
    user: RUser,
    mut conn: AppDbConn,
    part: String,
) -> ApiResult<Part> {
    dbg!(&part);
    let part = serde_json::from_str::<ChangePart>(&part)?;
    Ok(part.change(&user, &mut conn).map(Json)?)
}
