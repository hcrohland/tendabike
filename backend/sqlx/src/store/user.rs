use sqlx::FromRow;

use crate::{SqlxConn, into_domain};
use tb_domain::{OnboardingStatus, TbResult, User, UserId};

#[derive(Clone, Debug, FromRow)]
pub struct DbUser {
    id: i32,
    name: String,
    firstname: String,
    is_admin: bool,
    avatar: Option<String>,
    onboarding_status: i32,
}

impl From<User> for DbUser {
    fn from(value: User) -> Self {
        let User {
            id,
            name,
            firstname,
            avatar,
            is_admin,
            onboarding_status,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            avatar,
            is_admin,
            onboarding_status: onboarding_status.into(),
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
            onboarding_status,
        } = value;
        Self {
            id: id.into(),
            name,
            firstname,
            avatar,
            is_admin,
            onboarding_status: OnboardingStatus::try_from(onboarding_status)
                .expect("Invalid onboarding status in database"),
        }
    }
}

#[async_trait::async_trait]
impl<'c> tb_domain::UserStore for SqlxConn<'c> {
    async fn get(&mut self, uid: UserId) -> TbResult<User> {
        sqlx::query_as!(
            DbUser,
            "SELECT id, name, firstname, is_admin, avatar, onboarding_status FROM users WHERE id = $1",
            i32::from(uid)
        )
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
        let onboarding_status = i32::from(OnboardingStatus::Pending);
        sqlx::query_as!(
            DbUser,
            "INSERT INTO users (firstname, name, is_admin, avatar, onboarding_status)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
            firstname_,
            lastname,
            false,
            avatar_ as _,
            onboarding_status
        )
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
        sqlx::query_as!(
            DbUser,
            "UPDATE users
             SET firstname = $2, name = $3, avatar = $4
             WHERE id = $1
             RETURNING *",
            i32::from(*uid),
            firstname_,
            lastname,
            avatar_ as _
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> {
        let result = sqlx::query!("DELETE FROM users WHERE id = $1", i32::from(*user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn update_onboarding_status(
        &mut self,
        uid: &UserId,
        status: OnboardingStatus,
    ) -> TbResult<User> {
        sqlx::query_as!(
            DbUser,
            "UPDATE users
             SET onboarding_status = $2
             WHERE id = $1
             RETURNING *",
            i32::from(*uid),
            i32::from(status)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }
}
