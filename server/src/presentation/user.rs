use rocket::{Outcome, request::{FromRequest, self}, Request, http::Status};

use crate::{domain::user::{User, Person}, drivers::strava::error::TbResult};

struct RUser<'a> ( &'a User );

fn readuser (request: &Request) -> TbResult<User> {
    let id = crate::drivers::strava::auth::get_id(request)?;
    let conn = request.guard::<super::AppDbConn>().expect("No db request guard").0;
    User::read(id, &conn)
}

impl<'a, 'r> FromRequest<'a, 'r> for RUser<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> rocket::Outcome<RUser<'a>, (rocket::http::Status, &'a anyhow::Error), ()> {
        let user_result = request.local_cache(|| readuser(request));

        match user_result.as_ref() {
            Ok(x) => Outcome::Success(RUser(x)),
            Err(e) => Outcome::Failure((Status::Unauthorized, e)),
        }
    }
}

pub struct Admin<'a> (&'a User);

impl<'a, 'r> FromRequest<'a, 'r> for Admin<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Admin<'a>, &'a anyhow::Error> {
        let RUser(user) = request.guard::<RUser>()?;

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

