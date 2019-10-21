use crate::*;
use std::collections::HashMap;
use serde_json::Value;

#[allow(clippy::map_entry)]
#[get("/")]
fn dash (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();
    let res:Value = serde_json::from_str(&(user.request("/part/mygear"))?)?;
    map.insert("gear", dbg!(res));
    Ok(Template::render("dashboard", map))
}


pub fn routes () -> Vec<rocket::Route> {
    routes![dash
    ]
}