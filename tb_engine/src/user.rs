use crate::*;
use anyhow::Context;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::response::status;
use rocket::Outcome;
use rocket_contrib::json::Json;
use schema::*;

pub trait Person {
    fn get_id(&self) -> i32;
    fn is_admin(&self) -> bool;
    fn check_owner(&self, owner: i32, error: String) -> TbResult<()> {
        if self.get_id() == owner || self.is_admin() {
            Ok(())
        } else {
            Err(Error::Forbidden(error).into())
        }
    }
}

#[derive(Clone, Debug, Queryable, Insertable, Serialize, Deserialize)]
pub struct User {
    id: i32,
    name: String,
    firstname: String,
    is_admin: bool,
}

impl User {
    fn read(request: &Request) -> TbResult<Self> {
        let id = token::id(request)?;
        let conn = request.guard::<AppDbConn>().expect("No db request guard").0;
        Ok(users::table.find(id).get_result(&conn)?)
    }
}

impl Person for User {
    fn get_id(&self) -> i32 {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}

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

pub struct Admin<'a> {
    user: &'a User,
}

impl<'a, 'r> FromRequest<'a, 'r> for Admin<'a> {
    type Error = &'a anyhow::Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Admin<'a>, &'a anyhow::Error> {
        let user = request.guard::<&User>()?;

        if user.is_admin {
            Outcome::Success(Admin { user })
        } else {
            Outcome::Forward(())
        }
    }
}

impl Person for Admin<'_> {
    fn get_id(&self) -> i32 {
        self.user.id
    }
    fn is_admin(&self) -> bool {
        true
    }
}

#[get("/")]
fn getuser(user: &User) -> Json<&User> {
    Json(user)
}

#[derive(Deserialize)]
struct NewUser {
    #[serde(alias = "lastname")]
    name: String,
    firstname: String,
}

#[post("/", data = "<user>")]
fn post(user: Json<NewUser>, conn: AppDbConn) -> Result<status::Created<Json<User>>, ApiError> {
    use schema::users::dsl::*;

    let user: User = diesel::insert_into(users)
        .values((
            name.eq(&user.name),
            firstname.eq(&user.firstname),
            is_admin.eq(false),
        ))
        .get_result(&conn.0)
        .context("Could not create user")?;
    Ok(status::Created("/".to_string(), Some(Json(user))))
}

#[post("/echo", data = "<user>")]
fn echo(user: String) -> String {
    user
}

#[get("/test")]
fn test(_user: Admin, conn: AppDbConn) -> Result<&'static str, diesel::result::Error> {
    conn.0.begin_test_transaction()?;
    let msg = "Test Transaction started";
    warn!("{}", msg);
    Ok(msg)
}

pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, post, test, echo]
}
