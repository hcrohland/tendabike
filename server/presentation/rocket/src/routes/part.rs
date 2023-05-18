
use super::*;
use crate::domain::{ChangePart, NewPart, Part, PartId};

#[get("/<part>")]
fn get(part: i32, user: RUser, mut conn: AppDbConn) -> ApiResult<Part> {
    PartId::new(part).part(&user, &mut conn).map(Json)
}

#[post("/", data = "<newpart>")]
fn post(
    newpart: Json<NewPart>,
    user: RUser,
    mut conn: AppDbConn,
) -> Result<status::Created<Json<Part>>, ApiError> {
    let part = newpart.clone().create(&user, &mut conn)?;
    let url = rocket::uri!(get: i32::from(part.id));
    Ok(status::Created(url.to_string(), Some(Json(part))))
}

#[put("/", data = "<part>")]
fn put(
    part: Json<ChangePart>,
    user: RUser,
    mut conn: AppDbConn,
) -> ApiResult<Part> {
    part.change(&user, &mut conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, post, put]
}
