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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onboarding_status_roundtrips_through_i32() {
        for status in [
            OnboardingStatus::Pending,
            OnboardingStatus::InitialSyncPostponed,
            OnboardingStatus::Completed,
        ] {
            assert_eq!(
                OnboardingStatus::try_from(i32::from(status)).unwrap(),
                status
            );
        }
        assert!(matches!(
            OnboardingStatus::try_from(1),
            Err(tb_domain::Error::BadRequest(msg)) if msg == "Invalid onboarding status: 1"
        ));
    }

    #[test]
    fn user_db_roundtrip_preserves_fields() {
        let user = User {
            id: UserId::from(7),
            name: "Bike".to_string(),
            firstname: "Tenda".to_string(),
            avatar: Some("pic.png".to_string()),
            is_admin: false,
            onboarding_status: OnboardingStatus::Completed,
        };
        let db = DbUser::from(user);
        assert_eq!(db.id, 7);
        assert_eq!(db.name, "Bike");
        assert_eq!(db.firstname, "Tenda");
        assert_eq!(db.avatar.as_deref(), Some("pic.png"));
        assert!(!db.is_admin);
        assert_eq!(db.onboarding_status, 99);
        let back = User::from(db);
        assert_eq!(back.id, UserId::from(7));
        assert_eq!(back.name, "Bike");
        assert_eq!(back.firstname, "Tenda");
        assert_eq!(back.avatar.as_deref(), Some("pic.png"));
        assert!(!back.is_admin);
        assert_eq!(back.onboarding_status, OnboardingStatus::Completed);
    }

    #[test]
    #[should_panic(expected = "Invalid onboarding status in database")]
    fn user_from_db_panics_on_unknown_onboarding_status() {
        let db = DbUser {
            id: 1,
            name: "Bike".to_string(),
            firstname: "Tenda".to_string(),
            avatar: None,
            is_admin: false,
            onboarding_status: 5,
        };
        let _ = User::from(db);
    }
}
