
use rocket::response::status;
use rocket_contrib::json::Json;

use super::*;
use domain::activity::*;

#[post("/defaultgear", data="<gearid>")]
fn def_part_api (gearid: Json<PartId>, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(conn.transaction(|| {
        Ok(def_part(&gearid, user.0, &conn)?)
    }))
}

#[get("/rescan")]
fn rescan(_u: Admin, conn: AppDbConn) -> ApiResult<()> {
    let conn = &conn.0;
    warn!("rescanning all activities!");
    let res = conn.transaction(|| {
        {
            use schema::parts::dsl::*;
            debug!("resetting all parts");
            diesel::update(parts).set((
                time.eq(0),
                distance.eq(0),
                climb.eq(0),
                descend.eq(0),
                count.eq(0),
            )).execute(conn)?;
        }
        {
            use schema::attachments::dsl::*;
            debug!("resetting all attachments");
            diesel::update(attachments).set((
                time.eq(0),
                distance.eq(0),
                climb.eq(0),
                descend.eq(0),
                count.eq(0),
            )).execute(conn)?;
        }
        {
            use schema::activities::dsl::*;
            for a in activities.order_by(id).get_results::<Activity>(conn)? {
                debug!("registering activity {}", a.id);
                a.register(Factor::Add, conn)?;
            }
        }
        Ok(())
    });
    warn!("Done rescanning");
    tbapi(res)
}


/// web interface to read an activity
#[get("/<id>")]
fn get(id: i32, user: RUser, conn: AppDbConn) -> ApiResult<Activity> {
    tbapi(ActivityId::new(id).read(user.0, &conn))
}

/// web interface to create an activity
#[post("/", data = "<activity>")]
fn post(
    activity: Json<NewActivity>,
    user: RUser,
    conn: AppDbConn,
) -> Result<status::Created<Json<Summary>>, ApiError> {
    let assembly = Activity::create(&activity, user.0, &conn)?;
    let id_raw: i32 = assembly.activities[0].id.into();
    let url = uri!(get: id_raw);
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
) -> Result<Json<Summary>, ApiError> {
    tbapi(ActivityId::new(id).update(&activity, user.0, &conn))
}

/// web interface to delete an activity
#[delete("/<id>")]
fn delete(id: i32, user: RUser, conn: AppDbConn) -> ApiResult<Summary> {
    tbapi(ActivityId::new(id).delete(user.0, &conn))
}

#[post("/descend?<tz>", data = "<data>")]
fn descend(data: rocket::data::Data, tz: String, user: RUser, conn: AppDbConn) -> ApiResult<(Summary, Vec<String>, Vec<String>)> {
    tbapi(csv2descend(data.open(), tz, user.0, &conn))
}

#[get("/categories")]
fn mycats(user: RUser, conn: AppDbConn) -> ApiResult<Vec<PartTypeId>> {
    tbapi(categories(user.0, &conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![get, put, delete, post, descend, mycats, rescan, def_part_api]
}
