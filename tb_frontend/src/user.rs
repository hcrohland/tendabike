use rocket::Outcome;
use rocket::http::{Status};
use rocket::request::{self, Request, FromRequest};
use reqwest::{Body, Method};
use crate::*;

const ENGINE_URI: &str = "http://localhost:8000";

pub struct User (pub String);
impl User {
    fn get(request: &Request) -> TbResult<User> {
        let token = token::token(request)?;
        Ok(User(token))
    }

    /// send an API call with an authenticated User
    /// 
    pub fn get_request(&self, uri: &str) -> TbResult<serde_json::Value> {
        let client = reqwest::Client::new();
        Ok(client.get(&format!("{}{}", ENGINE_URI, uri))
            .bearer_auth(&self.0)
            .send().context("Could not reach engine")?
            .error_for_status()?
            .json().context("Could not get response body")?)
    }

    pub fn request<T: Into<Body>>(&self, method: Method, uri: &str, body: T) -> TbResult<serde_json::Value> {
        let client = reqwest::Client::new();
        Ok(client.request(method, &format!("{}{}", ENGINE_URI, uri))
            .bearer_auth(&self.0)
            .body(body)
            .send().context("Could not reach engine")?
            .error_for_status()?
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