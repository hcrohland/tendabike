use crate::*;
use auth::User;
use activity::*;

fn next_activities(user: &User, per_page: Option<i32>) -> TbResult<Vec<StravaActivity>> {

    let r = user.request(&format!("/activities?after={}&per_page={}", user.last_activity(), per_page.unwrap_or(10)))?;
    // let r = user.request("/activities?per_page=2")?;
    let acts: Vec<StravaActivity> = serde_json::from_str(&r)?;
    Ok(acts)
}

#[get("/next?<batch>")]
fn next (batch: Option<i32>, user: User) -> ApiResult<Vec<TbActivity>> {
    tbapi(next_activities(&user, batch)?.into_iter().map(|a| a.into_tb(&user)).collect())
}

#[get("/sync?<batch>")]
fn sync (batch: Option<i32>, user: User) -> ApiResult<Vec<serde_json::Value>> {
    let acts = next_activities(&user, batch)?;
   
    tbapi(acts.into_iter()
        .map(|a| a.send_to_tb(&user))
        .collect::<TbResult<Vec<_>>>())
}

pub fn routes () -> Vec<rocket::Route> {
    routes![sync, next
    ]
}