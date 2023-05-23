use super::*;
use schema::users;

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
    pub fn read(id:i32, conn: &mut AppConn) -> AnyResult<Self> {
        Ok(users::table.find(id).get_result(conn)?)
    }

    pub fn get_stat(uid: i32, conn: &mut AppConn) -> AnyResult<Stat> {
        let user = User::read(uid, conn)?;
        let parts = {
            use schema::parts::dsl::{parts, owner};
            parts.count().filter(owner.eq(uid)).first(conn)?
        };
        let activities = {
            use schema::activities::dsl::*;
            activities.count().filter(user_id.eq(uid)).first(conn)?
        };
        Ok(Stat{user, parts, activities})
    }  

    pub fn create(forename: &str, lastname: &str, conn: &mut AppConn) -> AnyResult<i32> {
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

    pub fn names(&self) -> (&str, &str) {
        (&self.firstname, &self.name)
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