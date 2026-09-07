use std::collections::HashMap;

use log::error;
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

#[async_trait::async_trait]
impl<'c> tb_strava::StravaStore for SqlxConn<'c> {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        sqlx::query_scalar!("SELECT id FROM strava_users WHERE tendabike_id = $1", who)
            .fetch_one(&mut **self.inner())
            .await
            .map_err(into_domain)
    }

    async fn stravaevent_store(&mut self, e: Event) -> TbResult<()> {
        let e: DbEvent = e.into();
        sqlx::query_as!(
            DbEvent,
            r#"INSERT INTO strava_events (object_type, object_id, aspect_type, updates, owner_id, subscription_id, event_time)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, object_type as "object_type!", object_id, aspect_type as "aspect_type!", updates as "updates!", owner_id, subscription_id, event_time"#,
            e.object_type,
            e.object_id,
            e.aspect_type,
            e.updates,
            e.owner_id,
            e.subscription_id,
            e.event_time
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        sqlx::query!("DELETE FROM strava_events WHERE id = $1", event_id)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        sqlx::query!(
            "UPDATE strava_events SET event_time = $2 WHERE id = $1",
            e_id,
            e_time
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn strava_event_get_next_for_user(
        &mut self,
        user_id: StravaId,
    ) -> TbResult<Option<Event>> {
        sqlx::query_as!(
            DbEvent,
            r#"SELECT id, object_type as "object_type!", object_id, aspect_type as "aspect_type!", updates as "updates!", owner_id, subscription_id, event_time FROM strava_events
             WHERE owner_id = ANY($1)
             ORDER BY event_time ASC
             LIMIT 1"#,
            &vec![0, user_id.into()] as _
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        sqlx::query_as!(
            DbEvent,
            r#"SELECT id, object_type as "object_type!", object_id, aspect_type as "aspect_type!", updates as "updates!", owner_id, subscription_id, event_time FROM strava_events
             WHERE object_id = $1 AND owner_id = $2
             ORDER BY event_time ASC"#,
            obj_id,
            i32::from(oid)
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        sqlx::query!("DELETE FROM strava_events WHERE id = ANY($1)", values as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        sqlx::query_as!(DbStravaUser, "SELECT * FROM strava_users")
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        sqlx::query_as!(
            DbStravaUser,
            "SELECT * FROM strava_users WHERE tendabike_id = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        sqlx::query_as!(
            DbStravaUser,
            "SELECT * FROM strava_users WHERE id = $1",
            i32::from(*id)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        let db_user = DbStravaUser::from(user);
        sqlx::query_as!(
            DbStravaUser,
            "INSERT INTO strava_users (id, tendabike_id, refresh_token)
             VALUES ($1, $2, $3)
             RETURNING *",
            db_user.id,
            db_user.tendabike_id,
            db_user.refresh_token as _
        )
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
        sqlx::query_as!(
            DbStravaUser,
            "UPDATE strava_users
             SET refresh_token = $2
             WHERE id = $1
             RETURNING *",
            i32::from(stravaid),
            refresh
        )
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
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM strava_events WHERE owner_id = $1",
            i32::from(*user)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(count.unwrap_or(0))
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        let result = sqlx::query!(
            "DELETE FROM strava_events WHERE owner_id = $1",
            i32::from(*user)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        let result = sqlx::query!(
            "DELETE FROM strava_users WHERE tendabike_id = $1",
            i32::from(user)
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strava_user(token: Option<&str>) -> StravaUser {
        StravaUser {
            id: StravaId::from(42),
            tendabike_id: UserId::from(1),
            refresh_token: token.map(|t| RefreshToken::new(t.to_string())),
        }
    }

    #[test]
    fn strava_user_refresh_token_roundtrips() {
        let user = strava_user(Some("secret-token"));
        let db = DbStravaUser::from(user.clone());
        assert_eq!(db.id, 42);
        assert_eq!(db.tendabike_id, 1);
        assert_eq!(db.refresh_token.as_deref(), Some("secret-token"));
        let back = StravaUser::from(db);
        assert_eq!(
            back.refresh_token.map(|t| t.secret().to_string()),
            Some("secret-token".to_string())
        );
    }

    #[test]
    fn strava_user_without_refresh_token_roundtrips() {
        let user = strava_user(None);
        let db = DbStravaUser::from(user);
        assert_eq!(db.refresh_token, None);
        assert!(StravaUser::from(db).refresh_token.is_none());
    }

    #[test]
    fn db_strava_user_and_event_defaults() {
        let user = DbStravaUser::default();
        assert_eq!(user.id, 0);
        assert_eq!(user.tendabike_id, 0);
        assert_eq!(user.refresh_token, None);
        let event = DbEvent::default();
        assert_eq!(event.id, None);
        assert_eq!(event.object_type, "");
        assert_eq!(event.object_id, 0);
        assert_eq!(event.aspect_type, "");
        assert_eq!(event.updates, "");
        assert_eq!(event.owner_id, 0);
        assert_eq!(event.subscription_id, 0);
        assert_eq!(event.event_time, 0);
    }

    #[test]
    fn event_updates_roundtrip_through_json() {
        let mut updates = HashMap::new();
        updates.insert("title".to_string(), "Morning Ride".to_string());
        updates.insert("private".to_string(), "false".to_string());
        let event = Event {
            id: Some(7),
            object_type: tb_strava::event::ObjectType::try_from("activity".to_string()).unwrap(),
            object_id: 99,
            aspect_type: tb_strava::event::AspectType::try_from("update".to_string()).unwrap(),
            updates,
            owner_id: StravaId::from(42),
            subscription_id: 5,
            event_time: 1700000000,
        };
        let db = DbEvent::from(event.clone());
        assert_eq!(db.object_type, "activity");
        assert_eq!(db.aspect_type, "update");
        assert_eq!(db.owner_id, 42);
        assert_eq!(
            serde_json::from_str::<HashMap<String, String>>(&db.updates).unwrap(),
            event.updates
        );
        assert_eq!(Event::from(db).updates, event.updates);
    }

    fn db_event(object_type: &str, aspect_type: &str, updates: &str) -> DbEvent {
        DbEvent {
            object_type: object_type.to_string(),
            aspect_type: aspect_type.to_string(),
            updates: updates.to_string(),
            ..Default::default()
        }
    }

    #[test]
    #[should_panic(expected = "Unknown object type nonsense")]
    fn db_event_unknown_object_type_panics() {
        let _ = Event::from(db_event("nonsense", "update", "{}"));
    }

    #[test]
    #[should_panic(expected = "Unknown aspect type bogus")]
    fn db_event_unknown_aspect_type_panics() {
        let _ = Event::from(db_event("activity", "bogus", "{}"));
    }

    #[test]
    #[should_panic(expected = "expected ident")]
    fn db_event_invalid_updates_json_panics() {
        let _ = Event::from(db_event("activity", "update", "not json"));
    }
}
