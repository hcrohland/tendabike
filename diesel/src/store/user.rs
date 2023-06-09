use tb_domain::AnyResult;
use crate::AppConn;
use tb_domain::User;
use tb_domain::UserId;
use anyhow::Context;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use tb_domain::schema;

#[async_session::async_trait]
impl tb_domain::UserStore for AppConn {
    async fn user_read_by_id(&mut self, uid: UserId) -> AnyResult<User> {
        schema::users::table
            .find(uid)
            .get_result(self)
            .await
            .context("error loading user")
    }

    async fn user_create(&mut self, firstname_: &str, lastname: &str) -> AnyResult<User> {
        use schema::users::dsl::*;
    
        diesel::insert_into(users)
            .values((
                firstname.eq(firstname_),
                name.eq(lastname),
                is_admin.eq(false),
            ))
            .get_result(self)
            .await
            .context("Could not create user")
    }
    
    async fn user_update(&mut self, uid: &UserId, firstname_: &str, lastname: &str) -> Result<User, anyhow::Error> {
        use schema::users::dsl::*;
        diesel::update(users.filter(id.eq(uid)))
            .set((firstname.eq(firstname_), name.eq(lastname)))
            .get_result(self)
            .await
            .context("Could not update user")
    }

    
}
