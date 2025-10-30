use sqlx::FromRow;

use crate::{SqlxConn, into_domain};
use tb_domain::{TbResult, User, UserId};

#[derive(Clone, Debug, FromRow)]
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
impl tb_domain::UserStore for SqlxConn {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        sqlx::query_as::<_, DbUser>("SELECT * FROM users WHERE id = $1")
            .bind(i32::from(uid))
            .fetch_one(&mut **self.inner())
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
        sqlx::query_as::<_, DbUser>(
            "INSERT INTO users (firstname, name, is_admin, avatar)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(firstname_)
        .bind(lastname)
        .bind(false)
        .bind(avatar_)
        .fetch_one(&mut **self.inner())
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
        sqlx::query_as::<_, DbUser>(
            "UPDATE users
             SET firstname = $2, name = $3, avatar = $4
             WHERE id = $1
             RETURNING *",
        )
        .bind(i32::from(*uid))
        .bind(firstname_)
        .bind(lastname)
        .bind(avatar_)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(i32::from(*user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
