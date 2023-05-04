use super::*;
use rocket_contrib::json::Json;
use crate::{domain::types::*};

// get all activity types
#[get("/activity")]
fn activity(_user: RUser, conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(activities(&conn))
}

/// get all part types
#[get("/part")]
fn part(conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(parts(&conn))
}

pub(super) fn routes() -> Vec<rocket::Route> {
    routes![part, activity]
}
