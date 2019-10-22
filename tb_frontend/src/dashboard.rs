use crate::*;
use std::collections::HashMap;

#[allow(clippy::map_entry)]
#[get("/")]
fn dash (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.request("/types/part")?);
    map.insert("user", user.request("/user")?);
    map.insert("gear", user.request("/part/mygear")?);
    map.insert("spares", user.request("/part/myspares")?);
    
    Ok(Template::render("dashboard", map))
}


pub fn routes () -> Vec<rocket::Route> {
    routes![dash
    ]
}