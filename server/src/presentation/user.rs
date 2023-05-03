use rocket::Outcome;


use super::*;

use crate::services::user::{User, Person};

impl<'a, 'r> FromRequest<'a, 'r> for &'a User {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<&'a User, &'a anyhow::Error> {
        let user_result = request.local_cache(|| User::read(request));

        match user_result.as_ref() {
            Ok(x) => Outcome::Success(x),
            Err(e) => Outcome::Failure((Status::Unauthorized, e)),
        }
    }
}

pub struct Admin<'a> (&'a User);

impl<'a, 'r> FromRequest<'a, 'r> for Admin<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Admin<'a>, &'a anyhow::Error> {
        let user = request.guard::<&User>()?;

        if user.is_admin() {
            Outcome::Success(Admin(user))
        } else {
            Outcome::Forward(())
        }
    }
}

impl Person for Admin<'_> {
    fn get_id(&self) -> i32 {
        self.0.get_id()
    }
    fn is_admin(&self) -> bool {
        assert!(self.0.is_admin());
        true
    }
}

