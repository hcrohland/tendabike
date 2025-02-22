use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use crate::{AsyncDieselConn, into_domain};
use tb_domain::{TbResult, User, UserId, schema};

#[async_session::async_trait]
impl tb_domain::UserStore for AsyncDieselConn {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        schema::users::table
            .find(uid)
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn create(&mut self, firstname_: &str, lastname: &str) -> TbResult<User> {
        use schema::users::dsl::*;

        diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result(self)
            .await
            .map_err(into_domain)
    }

    async fn update(&mut self, uid: &UserId, firstname_: &str, lastname: &str) -> TbResult<User> {
        use schema::users::dsl::*;
        diesel::update(users.filter(id.eq(uid)))
            .set((firstname.eq(firstname_), name.eq(lastname)))
            .get_result(self)
            .await
            .map_err(into_domain)
    }
}
