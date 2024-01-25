use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use crate::{map_to_tb, AsyncDieselConn};
use tb_domain::{schema, TbResult, User, UserId};

#[async_session::async_trait]
impl tb_domain::UserStore for AsyncDieselConn {
    async fn user_read_by_id(&mut self, uid: UserId) -> TbResult<User> {
        schema::users::table
            .find(uid)
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn user_create(&mut self, firstname_: &str, lastname: &str) -> TbResult<User> {
        use schema::users::dsl::*;

        diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }

    async fn user_update(
        &mut self,
        uid: &UserId,
        firstname_: &str,
        lastname: &str,
    ) -> TbResult<User> {
        use schema::users::dsl::*;
        diesel::update(users.filter(id.eq(uid)))
            .set((firstname.eq(firstname_), name.eq(lastname)))
            .get_result(self)
            .await
            .map_err(map_to_tb)
    }
}
