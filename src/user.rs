use rocket::Outcome;
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};

pub trait Person {
    fn get_id(&self) -> i32;
}

pub struct User (pub i32);

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = i32;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, i32> {
        let keys: Vec<_> = request.headers().get("x-user-id").collect();
        match keys.len() {
            0 => Outcome::Success(User(0)),
            1 => match keys[0].parse() {
                    Ok(1) => Outcome::Failure((Status::BadRequest, 3)),

                    Ok(id) => Outcome::Success(User(id)),
                    _ => Outcome::Failure((Status::BadRequest, 2)),
                }
            _ => Outcome::Failure((Status::BadRequest, 1)),
        }
    }
}

pub struct Admin (pub i32);

impl<'a, 'r> FromRequest<'a, 'r> for Admin {
    type Error = i32;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, i32> {
        let keys: Vec<_> = request.headers().get("x-user-id").collect();
        match keys.len() {
//            0 => Outcome::Success(Admin(1)),
            1 => match keys[0].parse() {
                    Ok(1) => Outcome::Success(Admin(1)),
                    _ => Outcome::Forward(()),
                }
            _ => Outcome::Failure((Status::BadRequest, 1)),
        }
    }
}

impl Person for User {
     fn get_id(&self) -> i32 {
        self.0
    }
}

impl Admin {
    pub fn get_id (&self) -> i32 {
        self.0
    }
}