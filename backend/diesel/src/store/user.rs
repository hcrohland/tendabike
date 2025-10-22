use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use super::schema;
use crate::{AsyncDieselConn, into_domain};
use tb_domain::{TbResult, User, UserId};
#[derive(Clone, Debug, Queryable, Insertable)]
#[diesel(table_name = schema::users)]
pub struct DbUser {
    id: i32,
    name: String,
    firstname: String,
    is_admin: bool,
    avatar: Option<String>,
}

impl From<User> for DbUser {
    fn from(value: User) -> Self {
        let User {
            id,
            name,
            firstname,
            avatar,
            is_admin,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            avatar,
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
            avatar,
            is_admin,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            avatar,
            is_admin,
        }
    }
}

#[async_session::async_trait]
impl tb_domain::UserStore for AsyncDieselConn {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        schema::users::table
            .find(i32::from(uid))
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn create(
        &mut self,
        firstname_: &str,
        lastname: &str,
        avatar_: &Option<String>,
    ) -> TbResult<User> {
        use schema::users::dsl::*;

        diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
                avatar.eq(avatar_),
            ))
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn update(
        &mut self,
        uid: &UserId,
        firstname_: &str,
        lastname: &str,
        avatar_: &Option<String>,
    ) -> TbResult<User> {
        use schema::users::dsl::*;
        diesel::update(users.find(i32::from(*uid)))
            .set((
                firstname.eq(firstname_),
                name.eq(lastname),
                avatar.eq(avatar_),
            ))
            .get_result::<DbUser>(self)
            .await
            .map_err(into_domain)
            .map(Into::into)
    }
}
