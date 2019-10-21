use crate::*;
use auth::User;
use rocket_contrib::templates::Template;
use std::collections::HashMap;

#[get("/plain")]
fn overview (user: User) -> ApiResult<String> {
    tbapi(user.request("/athlete"))
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
    Ok(Template::render("user", map))
}

use serde_json::Value;
#[get("/plain/activities")]
fn activities(user: User) -> ApiResult<Value> {
    tbapi(user.request_json("/athlete/activities?per_page=3"))
}

#[get("/plain/activity/<id>")]
fn activity(id: u64, user: User) -> ApiResult<Value> {
    tbapi(user.request_json(&format!("/activities/{}", id)))
}

#[get("/activities")]
fn activitiesh(user: User) -> TbResult<Template> {
    let res = user.request("/athlete/activities?per_page=3")?;
    let res: serde_json::Value = serde_json::from_str(&res)?;
    let mut map = HashMap::new();
    map.insert("activities", res);
    Ok(Template::render("activities", map))
}

#[get("/gear/<id>")]
fn gear(id: String, user: User) -> TbResult<Template> {
    let res = user.request(&format!("/gear/{}", &id))?;
    let res: serde_json::Value = serde_json::from_str(&res)?;
    let mut map = HashMap::new();
    map.insert("gear", res);
    Ok(Template::render("gear", map))
}

#[get("/plain/gear/<id>")]
fn gear_plain(id: String, user: User) -> ApiResult<Value> {
    tbapi(user.request_json(&format!("/gear/{}", &id)))
}

pub fn routes () -> Vec<rocket::Route> {
    routes![read, activities, gear, gear_plain, activitiesh, activity, overview
    ]
}