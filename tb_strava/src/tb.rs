use crate::*;
use auth::User;
use activity::*;

// use rocket_contrib::templates::Template;

use rocket_contrib::json::Json;

fn next_activities(user: &User) -> MyResult<Vec<TbActivity>> {

    let r = user.request(&format!("/activities?after={}&per_page=10", user.last_activity()))?;
    // let r = user.request("/activities?per_page=2")?;
    let acts: Vec<StravaActivity> = serde_json::from_str(&r).context(r)?;
 
    acts.into_iter().map(|a| a.into_tb(user)).collect()
}

#[get("/next")]
fn next (user: User) -> MyResult<Json<Vec<TbActivity>>> {
    Ok(Json(next_activities(&user)?))
}

#[get("/sync")]
fn sync (user: User) -> MyResult<Json<Vec<String>>> {
    let acts = next_activities(&user)?;
   
    acts.into_iter()
        .map(|a| a.send_to_tb(&user))
        .collect::<MyResult<Vec<String>>>()
        .map(Json)
}

pub fn routes () -> Vec<rocket::Route> {
    routes![sync, next
    ]
}