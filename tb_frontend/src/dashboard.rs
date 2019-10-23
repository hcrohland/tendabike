use crate::*;
use std::collections::HashMap;
use chrono::Utc;

#[get("/")]
fn dash (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.request("/types/part")?);
    map.insert("user", user.request("/user")?);
    map.insert("gear", user.request("/part/mygear")?);
    map.insert("spares", user.request("/part/myspares")?);
    
    Ok(Template::render("dashboard", map))
}


#[get("/part/<id>?<time>")]
fn part (id:i32, time: Option<String>, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    let param = parse_time(time)?.unwrap_or_else(Utc::now).format("%Y-%m-%dT%H:%M:%S").to_string();
    map.insert("types", user.request("/types/part")?);
    map.insert("gear", user.request(&format!("/part/{}?assembly&time={}", id, param))?);
    
    Ok(Template::render("part", map))
}

#[get("/part/history/<id>")]
fn part_hist (id:i32, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.request("/types/part")?);
    map.insert("attach", user.request(&format!("/attach/{}", id))?);
    
    Ok(Template::render("part_hist", map))
}



pub fn routes () -> Vec<rocket::Route> {
    routes![dash, part, part_hist
    ]
}