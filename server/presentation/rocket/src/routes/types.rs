use super::*;
use crate::domain::*;

// get all activity types
#[get("/activity")]
fn activity(_user: RUser, mut conn: AppDbConn) -> Json<Vec<ActivityType>> {
    Json(ActivityType::all_ordered(&mut conn))
}

/// get all part types
#[get("/part")]
fn part(mut conn: AppDbConn) -> Json<Vec<PartType>> {
    Json(PartType::all_ordered(&mut conn))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![part, activity]
}
