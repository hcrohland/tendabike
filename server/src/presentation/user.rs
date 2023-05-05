
use domain::user::Stat;
use rocket_contrib::json::Json;
use crate::drivers::strava;

use super::*;

#[get("/")]
fn getuser(user: RUser) -> Json<User> {
    Json(user.0.clone())
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<Stat>> {
    tbapi(User::get_all(&conn))
}

#[get("/summary")]
fn summary(context: strava::auth::StravaContext, conn: AppDbConn) -> ApiResult<Summary> {
    strava::ui::update_user(&context)?;
    let parts = domain::part::Part::get_all(&context, &conn)?;
    let attachments = Attachment::for_parts(&parts,&conn)?;
    let activities = Activity::get_all(&context, &conn)?;
    tbapi(Ok(Summary{parts,attachments,activities}))
}


pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}
