
use super::*;
use domain::{Activity, ActivityId, NewActivity, PartId, PartTypeId, Summary};

#[post("/defaultgear", data="<gear_id>")]
fn def_part_api (gear_id: Json<PartId>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    Activity::set_default_part(*gear_id, user.0, &conn).map(Json)
}

#[get("/rescan")]
fn rescan(_u: Admin, conn: AppDbConn) -> ApiResult<()> {
    let conn = &conn.0;
    Activity::rescan_all(&conn).map(Json)
}


/// web interface to read an activity
#[get("/<id>")]
fn get(id: i32, user: RUser, conn: AppDbConn) -> ApiResult<Activity> {
    ActivityId::new(id).read(user.0, &conn).map(Json)
}

/// web interface to create an activity
#[post("/", data = "<activity>")]
fn post(
    activity: Json<NewActivity>,
    user: RUser,
    conn: AppDbConn,
) -> Result<status::Created<Json<Summary>>, ApiError> {
    let assembly = Activity::create(&activity, user.0, &conn)?;
    let id_raw: i32 = assembly.first().into();
                    
    let url = rocket::uri!(get: id_raw);
    Ok(status::Created(
        url.to_string(),
        Some(Json(assembly)),
    ))
}

/// web interface to change an activity
#[put("/<id>", data = "<activity>")]
fn put(
    id: i32,
    activity: Json<NewActivity>,
    user: RUser,
    conn: AppDbConn,
) -> Result<Json<Summary>, anyhow::Error> {
    ActivityId::new(id).update(&activity, user.0, &conn).map(Json)
}

/// web interface to delete an activity
#[delete("/<id>")]
fn delete(id: i32, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    ActivityId::new(id).delete(user.0, &conn).map(Json)
}

#[post("/descend?<tz>", data = "<data>")]
fn descend(data: rocket::data::Data, tz: String, user: RUser, conn: AppDbConn) -> ApiResult<(Summary, Vec<String>, Vec<String>)> {
    Activity::csv2descend(data.open(), tz, user.0, &conn).map(Json)
}

#[get("/categories")]
fn mycats(user: RUser, conn: AppDbConn) -> ApiResult<Vec<PartTypeId>> {
    Activity::categories(user.0, &conn).map(Json)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, put, delete, post, descend, mycats, rescan, def_part_api]
}
