use crate::*;
use auth::User;
use rocket_contrib::templates::Template;
use std::collections::HashMap;
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

#[get("/user")]
fn overview (user: User) -> ApiResult<serde_json::Value> {
    tbapi(user.request_json("/athlete"))
}

#[allow(clippy::map_entry)]
#[get("/?<page>&<after>")]
fn read (page: Option<i32>, after: Option<i64>, user: User) -> TbResult<Template> {
    use serde_json::Value;
    let mut map = HashMap::new();

    let res = serde_json::from_str(&user.request("/athlete")?)?;
    map.insert("user", res);
    let page = page.unwrap_or(1);
    let res: Value = if let Some(after) = after {
        serde_json::from_str(&user.request(&format!("/activities?after={}&per_page={}", after, page))?)?
    } else {
        serde_json::from_str(&user.request(&format!("/activities?page={}", page))?)?
    };
    let mut gears = HashMap::new();
    for act in res.as_array().expect("No array") {
        let gear = act["gear_id"].as_str().unwrap_or("n/a");
        if !gears.contains_key(&gear){
            let res: Value = serde_json::from_str(&user.request(&format!("/gear/{}", &gear))?)?;
            gears.insert(gear, res);
        }
    }
    map.insert("gears", serde_json::to_value(gears)?);
    map.insert("activities", res);
    map.insert("page", serde_json::to_value(page)?);
    Ok(Template::render("strava_ui", map))
}

use serde_json::Value;
#[get("/activities")]
fn activities(user: User) -> ApiResult<Value> {
    tbapi(user.request_json("/athlete/activities?per_page=3"))
}

#[get("/activity/<id>")]
fn activity(id: u64, user: User) -> ApiResult<Value> {
    tbapi(user.request_json(&format!("/activities/{}", id)))
}

#[get("/gear/<id>")]
fn gear(id: String, user: User) -> ApiResult<Value> {
    tbapi(user.request_json(&format!("/gear/{}", &id)))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![read, activities, gear, activity, overview, sync, next
    ]
}