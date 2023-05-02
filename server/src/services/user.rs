use crate::*;
use anyhow::Context;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::Outcome;
use rocket_contrib::json::Json;
use schema::*;
use drivers::strava;

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

#[derive(Debug, Serialize)]
struct Stat {
    user: User,
    parts: i64,
    activities: i64,
    events: i64,
    disabled: bool,
}

impl User {
    fn read(request: &Request) -> TbResult<Self> {
        let id = strava::auth::get_id(request)?;
        let conn = request.guard::<AppDbConn>().expect("No db request guard").0;
        Ok(users::table.find(id).get_result(&conn)?)
    }

    fn get_stat(self, conn: &AppConn) -> TbResult<Stat> {
        let parts = {
            use schema::parts::dsl::{parts, owner};
            parts.count().filter(owner.eq(self.id)).first(conn)?
        };
        let activities = {
            use schema::activities::dsl::*;
            activities.count().filter(user_id.eq(self.id)).first(conn)?
        };
        let  (events, disabled) = strava::auth::User::get_stats(self.id, conn)?;
        Ok(Stat{user: self, parts, activities, events, disabled})
    }

    fn get_all (conn: &AppConn) -> TbResult<Vec<Stat>> {
        let users = users::table.get_results::<User>(conn)?;
        users.into_iter().map(|u| u.get_stat(conn)).collect::<TbResult<_>>()
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

pub fn create(forename: String, lastname: String, conn: &AppConn) -> TbResult<i32> {
    use crate::schema::users::dsl::*;

    let user: User = diesel::insert_into(users)
        .values((
            firstname.eq(forename),
            name.eq(lastname),
            is_admin.eq(false),
        ))
        .get_result(conn)
        .context("Could not create user")?;
    Ok(user.id)
}

#[get("/")]
fn getuser(user: &User) -> Json<&User> {
    Json(user)
}

#[get("/all")]
fn userlist(_u: Admin, conn: AppDbConn) -> ApiResult<Vec<Stat>> {
    tbapi(User::get_all(&conn))
}

#[get("/summary")]
fn summary(user: strava::auth::User, conn: AppDbConn) -> ApiResult<Summary> {
    strava::ui::update_user(&user)?;
    let parts = Part::get_all(&user, &conn)?;
    let attachments = Attachment::for_parts(&parts,&conn)?;
    let activities = Activity::get_all(&user, &conn)?;
    tbapi(Ok(Summary{parts,attachments,activities}))
}


pub fn routes() -> Vec<rocket::Route> {
    routes![getuser, userlist, summary]
}
