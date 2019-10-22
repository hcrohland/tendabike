use rocket::Outcome;
use rocket::http::{Status, Cookies};
use rocket::request::{self, Request, FromRequest};
use crate::*;

const ENGINE_URI: &str = "http://localhost:8000";

pub struct User (pub i32);
impl User {
    fn get(request: &Request) -> TbResult<User> {
        let mut cookies = request.guard::<Cookies>().expect("No request cookies!!!");
        let id = cookies.get_private("id").ok_or(Error::NotAuth("no id cookie"))?
                .value().parse::<i32>()?;
        Ok(User(id))
    }

    /// send an API call with an authenticated User
    /// 
    pub fn request(&self, uri: &str) -> TbResult<serde_json::Value> {
        let client = reqwest::Client::new();
        Ok(client.get(&format!("{}{}", ENGINE_URI, uri))
            .header("x-user-id", self.0)
            .send().context("Could not reach engine")?
            .json().context("Could not get response body")?)
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = i32;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, i32> {
        match User::get(request) {
            Ok(user) => Outcome::Success(user),
            Err(_) => Outcome::Failure((Status::Unauthorized, 1)),
        }
    }
}




pub fn routes () -> Vec<rocket::Route> {
    routes![
    ]
}