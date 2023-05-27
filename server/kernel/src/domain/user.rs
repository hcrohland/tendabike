use super::*;
use schema::users;

#[derive(DieselNewType, Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(i32);
NewtypeDisplay! { () pub struct UserId(); }
NewtypeFrom! { () pub struct UserId(i32); }

#[derive(Clone, Debug, Queryable, Insertable, Serialize, Deserialize)]
pub struct User {
    id: UserId,
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

impl UserId {
    pub fn read(self, conn: &mut AppConn) -> AnyResult<User> {
        Ok(users::table.find(self).get_result(conn)?)
    }

    pub fn get_stat(self, conn: &mut AppConn) -> AnyResult<Stat> {
        let user = self.read(conn)?;
        let parts = {
            use schema::parts::dsl::{parts, owner};
            parts.count().filter(owner.eq(self)).first(conn)?
        };
        let activities = {
            use schema::activities::dsl::*;
            activities.count().filter(user_id.eq(self)).first(conn)?
        };
        Ok(Stat{user, parts, activities})
    }  

    pub fn create(firstname_: &str, lastname: &str, conn: &mut AppConn) -> AnyResult<Self> {
        use crate::schema::users::dsl::*;
    
        let user: User = diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result(conn)
            .context("Could not create user")?;
        Ok(user.id)
    }

    pub fn update(&self, firstname_: &str, lastname: &str, conn: &mut AppConn) -> AnyResult<Self> {
        use crate::schema::users::dsl::*;
    
        let user: User = diesel::update(users.filter(id.eq(self)))
            .set((
                firstname.eq(firstname_),
                name.eq(lastname),
            ))
            .get_result(conn)
            .context("Could not update user")?;
        Ok(user.id)
    }

    pub fn is_admin(&self, conn: &mut r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>) -> AnyResult<bool> {
        self.read(conn).map(|u| u.is_admin)
    }     
}

impl Person for User {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}