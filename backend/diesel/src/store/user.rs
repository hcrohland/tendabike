use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{TbResult, User, UserId, schema};

#[derive(Clone, Debug, Queryable, Insertable)]
#[diesel(table_name = schema::users)]
pub struct DbUser {
    id: i32,
    name: String,
    firstname: String,
    is_admin: bool,
}

impl From<User> for DbUser {
    fn from(value: User) -> Self {
        let User {
            id,
            name,
            firstname,
            is_admin,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            is_admin,
        }
    }
}

impl From<DbUser> for User {
    fn from(value: DbUser) -> Self {
        let DbUser {
            id,
            name,
            firstname,
            is_admin,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            is_admin,
        }
    }
}

#[async_session::async_trait]
impl tb_domain::UserStore for AsyncDieselConn {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        schema::users::table
            .find(uid.inner())
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn create(&mut self, firstname_: &str, lastname: &str) -> TbResult<User> {
        use schema::users::dsl::*;

        diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(&mut self, uid: &UserId, firstname_: &str, lastname: &str) -> TbResult<User> {
        use schema::users::dsl::*;
        diesel::update(users.filter(id.eq(uid.inner())))
            .set((firstname.eq(firstname_), name.eq(lastname)))
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }
}
