
use super::*;
use crate::domain::part::*;

#[get("/<part>")]
fn get(part: i32, user: RUser, conn: AppDbConn) -> ApiResult<Part> {
    PartId::new(part).part(user.0, &conn).map(Json)
}

#[post("/", data = "<newpart>")]
fn post(
    newpart: Json<NewPart>,
    user: RUser,
    conn: AppDbConn,
) -> Result<status::Created<Json<Part>>, ApiError> {
    let part = newpart.clone().create(user.0, &conn)?;
    let url = uri!(get: i32::from(part.id));
    Ok(status::Created(url.to_string(), Some(Json(part))))
}

#[put("/", data = "<part>")]
fn put(
    part: Json<ChangePart>,
    user: RUser,
    conn: AppDbConn,
) -> ApiResult<Part> {
    part.change(user.0, &conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, post, put]
}
