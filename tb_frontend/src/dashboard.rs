use crate::*;
use std::collections::HashMap;

#[get("/")]
fn dash (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.request("/types/part")?);
    map.insert("user", user.request("/user")?);
    map.insert("gear", user.request("/part/mygear")?);
    map.insert("spares", user.request("/part/myspares")?);
    
    Ok(Template::render("dashboard", map))
}


#[get("/part/<id>")]
fn part (id:i32, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.request("/types/part")?);
    map.insert("gear", user.request(&format!("/part/{}?assembly", id))?);
    
    Ok(Template::render("part", map))
}



pub fn routes () -> Vec<rocket::Route> {
    routes![dash, part
    ]
}