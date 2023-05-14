use std::ops::Deref;

use super::*;
pub struct RUser<'a> ( &'a User );

impl Deref for RUser<'_> {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Person for RUser<'_> {
    fn get_id(&self) -> i32 {
        self.0.get_id()
    }
    fn is_admin(&self) -> bool {
        assert!(self.0.is_admin());
        true
    }
}


fn readuser (request: &Request) -> AnyResult<User> {
let id = strava::get_id(request)?;
let conn = request.guard::<AppDbConn>().expect("No db request guard").0;
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
