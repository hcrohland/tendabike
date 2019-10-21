use crate::*;
use auth::User;
use activity::*;

fn next_activities(user: &User) -> TbResult<Vec<StravaActivity>> {

    let r = user.request(&format!("/activities?after={}&per_page=10", user.last_activity()))?;
    // let r = user.request("/activities?per_page=2")?;
    let acts: Vec<StravaActivity> = serde_json::from_str(&r)?;
    Ok(acts)
}

#[get("/next")]
fn next (user: User) -> ApiResult<Vec<TbActivity>> {
    tbapi(next_activities(&user)?.into_iter().map(|a| a.into_tb(&user)).collect())
}

#[get("/sync")]
fn sync (user: User) -> ApiResult<Vec<serde_json::Value>> {
    let acts = next_activities(&user)?;
   
    tbapi(acts.into_iter()
        .map(|a| a.send_to_tb(&user))
        .collect::<TbResult<Vec<_>>>())
}

pub fn routes () -> Vec<rocket::Route> {
    routes![sync, next
    ]
}