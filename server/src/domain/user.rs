use super::*;
use anyhow::Context;
use schema::users;

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
pub struct Stat {
    pub user: User,
    parts: i64,
    activities: i64,
}

impl User {
    pub fn read(id:i32, conn: &AppConn) -> TbResult<Self> {
        Ok(users::table.find(id).get_result(conn)?)
    }

    pub fn get_stat(id: i32, conn: &AppConn) -> TbResult<Stat> {
        let user = User::read(id, conn)?;
        let parts = {
            use schema::parts::dsl::{parts, owner};
            parts.count().filter(owner.eq(id)).first(conn)?
        };
        let activities = {
            use schema::activities::dsl::*;
            activities.count().filter(user_id.eq(id)).first(conn)?
        };
        Ok(Stat{user, parts, activities})
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
