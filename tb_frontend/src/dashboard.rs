use crate::*;
use std::collections::HashMap;
use chrono::{Utc, SecondsFormat};
use rocket::request::Form;
use rocket::response::Redirect;
use reqwest::Method;

#[get("/")]
fn dash (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.get_request("/types/part")?);
    map.insert("user", user.get_request("/user")?);
    map.insert("gear", user.get_request("/part/mygear")?);
    map.insert("spares", user.get_request("/part/myspares")?);
    
    Ok(Template::render("dashboard", map))
}


#[get("/part/<id>?<time>")]
fn part (id:i32, time: Option<String>, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();


    let time = parse_time(time)?.unwrap_or_else(Utc::now).to_rfc3339_opts(SecondsFormat::Secs, true);
    map.insert("time", json!(time));
    map.insert("types", user.get_request("/types/part")?);
    map.insert("main", user.get_request(&format!("/part/{}", id))?);
    map.insert("parts", user.get_request(&format!("/attach/assembly/{}?time={}", id, time))?);
    map.insert("attach", user.get_request(&format!("/attach/{}", id))?);
    
    Ok(Template::render("part", map))
}

#[get("/part/create")]
fn part_create (user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("types", user.get_request("/types/part")?);

    Ok(Template::render("part_new", map))
}

#[derive(Debug, FromForm, Serialize)]
struct NewPart {
    owner: Option<i32>,
    what: i32,
    name: String,
    vendor: String,
    model: String,
    purchase: String
}

#[post("/part/create", data="<data>")]
fn part_post (data: Form<NewPart>, user: User) -> TbResult<Redirect> {
    let mut data = data.into_inner();
    data.purchase.push_str("T12:00:00Z");
    let id = token::id_unsafe(&user.0)?;
    data.owner = Some(id);
    dbg!(&data);
    let res = user.request(Method::POST, "/part", serde_json::to_string(&data)?)?.as_i64().ok_or_else(|| anyhow!("No id returned)"))?;
    Ok(Redirect::to(format!("/part/{}", res)))
}

#[derive(Debug, FromForm, Serialize)]
struct Attachment {
    /// the sub-part, which is attached to the hook
    part_id: i32,
    /// when it was attached
    attached: String,
    #[serde(skip_serializing)]
    time: String,
    #[serde(skip_serializing)]
    tz: String,
    /// The gear the part is attached to
    gear: i32,
    /// the hook on that gear
    hook: i32,
}

#[post("/part/attach", data="<data>")]
fn part_attach (data: Form<Attachment>, user: User) -> TbResult<Redirect> {
    let mut data = data.into_inner();
    if data.attached.is_empty() {
        data.attached = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    } else if data.time.is_empty() {
        data.attached = data.attached + "T12:00:00" + &data.tz;
    } else {
        data.attached = data.attached + "T" + &data.time + ":00" + &data.tz;
    }
    user.request(Method::PATCH, "/attach", serde_json::to_string(&data)?)?;
    Ok(Redirect::to(format!("/part/{}", data.gear)))
}

#[get("/part/attach/<part>")]
fn create_attach (part: i32, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();

    map.insert("part", user.get_request(&format!("/part/{}", part))?);
    map.insert("types", user.get_request("/types/part")?);
    map.insert("gears", user.get_request("/part/mygear")?);
    
    Ok(Template::render("attach", map))
}

#[get("/type/<id>")]
fn parts_per_type (id:i32, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();
    map.insert("types", user.get_request("/types/part")?);
    map.insert("parts", user.get_request(&format!("/part/type/{}", id))?);
    
    Ok(Template::render("type", map))
}

#[get("/attached/<gear>/<what>")]
fn get_attached(gear: i32, what: i32, user: User) -> TbResult<Template> {
    let mut map = HashMap::new();
    map.insert("what", json!(what));
    map.insert("types", user.get_request("/types/part")?);
    map.insert("main", user.get_request(&format!("/part/{}", gear))?);
    map.insert("attach", user.get_request(&format!("/attach/check/{}/{}", gear, what))?);
    
    Ok(Template::render("attached", map))
}



pub fn routes () -> Vec<rocket::Route> {
    routes![dash, part, part_create, part_post, parts_per_type, get_attached, part_attach, create_attach
    ]
}