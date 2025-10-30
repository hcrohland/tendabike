use std::collections::HashMap;

use async_session::{log::error, serde_json};
use oauth2::RefreshToken;
use sqlx::FromRow;

use crate::{SqlxConn, into_domain, option_into, vec_into};
use tb_domain::{TbResult, UserId};
use tb_strava::{StravaId, StravaUser, event::Event};

#[derive(Clone, Debug, Default, FromRow)]
pub struct DbStravaUser {
    id: i32,
    tendabike_id: i32,
    refresh_token: Option<String>,
}

impl From<StravaUser> for DbStravaUser {
    fn from(value: StravaUser) -> Self {
        let StravaUser {
            id,
            tendabike_id,
            refresh_token,
        } = value;
        Self {
            id: id.into(),
            tendabike_id: tendabike_id.into(),
            refresh_token: refresh_token.map(RefreshToken::into_secret),
        }
    }
}
impl From<DbStravaUser> for StravaUser {
    fn from(value: DbStravaUser) -> Self {
        let DbStravaUser {
            id,
            tendabike_id,
            refresh_token,
        } = value;
        Self {
            id: id.into(),
            tendabike_id: tendabike_id.into(),
            refresh_token: refresh_token.map(RefreshToken::new),
        }
    }
}

#[derive(Debug, Default, FromRow)]
pub struct DbEvent {
    id: Option<i32>,
    pub object_type: String,
    pub object_id: i64,
    pub aspect_type: String,
    updates: String,
    owner_id: i32,
    subscription_id: i32,
    pub event_time: i64,
}

impl From<Event> for DbEvent {
    fn from(value: Event) -> Self {
        let Event {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id,
            subscription_id,
            event_time,
        } = value;
        let updates = serde_json::to_string(&updates).unwrap_or_else(|e| {
            error!("{e:?}");
            String::default()
        });
        Self {
            id,
            object_type: object_type.into(),
            object_id,
            aspect_type: aspect_type.into(),
            updates,
            owner_id: owner_id.into(),
            subscription_id,
            event_time,
        }
    }
}

impl From<DbEvent> for Event {
    fn from(value: DbEvent) -> Self {
        let DbEvent {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id,
            subscription_id,
            event_time,
        } = value;
        let object_type = object_type.try_into().unwrap();
        let aspect_type = aspect_type.try_into().unwrap();
        let updates: HashMap<String, String> = serde_json::from_str(&updates).unwrap();
        Self {
            id,
            object_type,
            object_id,
            aspect_type,
            updates,
            owner_id: owner_id.into(),
            subscription_id,
            event_time,
        }
    }
}

#[async_session::async_trait]
impl tb_strava::StravaStore for SqlxConn {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        sqlx::query_scalar::<_, i32>("SELECT id FROM strava_users WHERE tendabike_id = $1")
            .bind(who)
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
    }

    async fn stravaevent_store(&mut self, e: Event) -> TbResult<()> {
        let e: DbEvent = e.into();
        sqlx::query_as::<_, DbEvent>(
            "INSERT INTO strava_events (id, object_type, object_id, aspect_type, updates, owner_id, subscription_id, event_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING *"
        )
        .bind(e.id)
        .bind(e.object_type)
        .bind(e.object_id)
        .bind(e.aspect_type)
        .bind(e.updates)
        .bind(e.owner_id)
        .bind(e.subscription_id)
        .bind(e.event_time)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        sqlx::query("DELETE FROM strava_events WHERE id = $1")
            .bind(event_id)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        sqlx::query("UPDATE strava_events SET event_time = $2 WHERE id = $1")
            .bind(e_id)
            .bind(e_time)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_get_next_for_user(
        &mut self,
        user_id: StravaId,
    ) -> TbResult<Option<Event>> {
        sqlx::query_as::<_, DbEvent>(
            "SELECT * FROM strava_events
             WHERE owner_id = ANY($1)
             ORDER BY event_time ASC
             LIMIT 1",
        )
        .bind(vec![0, user_id.into()])
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        sqlx::query_as::<_, DbEvent>(
            "SELECT * FROM strava_events
             WHERE object_id = $1 AND owner_id = $2
             ORDER BY event_time ASC",
        )
        .bind(obj_id)
        .bind(i32::from(oid))
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        sqlx::query("DELETE FROM strava_events WHERE id = ANY($1)")
            .bind(values)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        sqlx::query_as::<_, DbStravaUser>("SELECT * FROM strava_users")
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        sqlx::query_as::<_, DbStravaUser>("SELECT * FROM strava_users WHERE tendabike_id = $1")
            .bind(i32::from(id))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(Into::into)
    }

    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        sqlx::query_as::<_, DbStravaUser>("SELECT * FROM strava_users WHERE id = $1")
            .bind(i32::from(*id))
            .fetch_optional(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(option_into)
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        let db_user = DbStravaUser::from(user);
        sqlx::query_as::<_, DbStravaUser>(
            "INSERT INTO strava_users (id, tendabike_id, refresh_token)
             VALUES ($1, $2, $3)
             RETURNING *",
        )
        .bind(db_user.id)
        .bind(db_user.tendabike_id)
        .bind(db_user.refresh_token)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        sqlx::query_as::<_, DbStravaUser>(
            "UPDATE strava_users
             SET refresh_token = $2
             WHERE id = $1
             RETURNING *",
        )
        .bind(i32::from(stravaid))
        .bind(refresh)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    /// return the open events and the disabled status for a user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM strava_events WHERE owner_id = $1")
            .bind(i32::from(*user))
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM strava_events WHERE owner_id = $1")
            .bind(i32::from(*user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM strava_users WHERE tendabike_id = $1")
            .bind(i32::from(user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
