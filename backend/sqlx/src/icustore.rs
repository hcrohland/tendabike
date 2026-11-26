use oauth2::AccessToken;
use sqlx::FromRow;

use crate::{SqlxConn, into_domain, option_into, vec_into};
use tb_domain::{TbResult, UserId};
use tb_provider::icu::{IcuEvent, IcuId, IcuUser};

#[derive(Clone, Debug, Default, FromRow)]
struct DbIcuUser {
    /// The serial row id: part of the row (for `RETURNING *`) but not used by
    /// the domain — `IcuUser.id` is the `icu_athlete_id`, not the serial.
    #[allow(dead_code)]
    id: i32,
    tbid: i32,
    icu_athlete_id: String,
    access_token: Option<String>,
}

impl From<IcuUser> for DbIcuUser {
    fn from(value: IcuUser) -> Self {
        let IcuUser {
            id,
            tendabike_id,
            access_token,
        } = value;
        Self {
            id: 0,
            tbid: tendabike_id.into(),
            icu_athlete_id: id.into(),
            // The typed token serializes to its secret string (the DB column).
            access_token: access_token.map(|t| t.secret().to_string()),
        }
    }
}

impl From<DbIcuUser> for IcuUser {
    fn from(value: DbIcuUser) -> Self {
        let DbIcuUser {
            id: _,
            tbid,
            icu_athlete_id,
            access_token,
        } = value;
        Self {
            id: icu_athlete_id.into(),
            tendabike_id: tbid.into(),
            // The DB column's secret string builds the typed token.
            access_token: access_token.map(AccessToken::new),
        }
    }
}

#[derive(Debug, Default, FromRow)]
struct DbIcuEvent {
    id: Option<i32>,
    icu_athlete_id: String,
    activity_id: String,
    event_time: i64,
    object_type: String,
    event_type: String,
    oldest: Option<i64>,
}

impl From<IcuEvent> for DbIcuEvent {
    fn from(value: IcuEvent) -> Self {
        let IcuEvent {
            id,
            icu_athlete_id,
            activity_id,
            event_time,
            object_type,
            event_type,
            oldest,
        } = value;
        Self {
            id,
            icu_athlete_id: icu_athlete_id.into(),
            activity_id,
            event_time,
            object_type: object_type.into(),
            event_type,
            oldest,
        }
    }
}

impl From<DbIcuEvent> for IcuEvent {
    fn from(value: DbIcuEvent) -> Self {
        let DbIcuEvent {
            id,
            icu_athlete_id,
            activity_id,
            event_time,
            object_type,
            event_type,
            oldest,
        } = value;
        let object_type = object_type.try_into().unwrap();
        Self {
            id,
            icu_athlete_id: icu_athlete_id.into(),
            activity_id,
            event_time,
            object_type,
            event_type,
            oldest,
        }
    }
}

/// The provider-agnostic store over the icu tables (spec #1029): the single
/// uniform [`Store`](tb_provider::Store) interface, generic over the
/// [`IcuSpec`](tb_provider::icu::IcuSpec) marker. `user_disable` is the
/// disable policy (drop the queued events, clear the token) and `limited_until`
/// the process-wide limiter.
#[async_trait::async_trait]
impl<'c> tb_provider::Store<tb_provider::icu::IcuSpec> for SqlxConn<'c> {
    async fn user_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        sqlx::query_as!(DbIcuUser, "SELECT * FROM icu_users")
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn user_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
        sqlx::query_as!(
            DbIcuUser,
            "SELECT * FROM icu_users WHERE tbid = $1",
            i32::from(id)
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn user_get_by_provider_id(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
        sqlx::query_as!(
            DbIcuUser,
            "SELECT * FROM icu_users WHERE icu_athlete_id = $1",
            id.to_string()
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn user_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        let db_user = DbIcuUser::from(user);
        sqlx::query_as!(
            DbIcuUser,
            "INSERT INTO icu_users (tbid, icu_athlete_id, access_token)
             VALUES ($1, $2, $3)
             RETURNING *",
            db_user.tbid,
            db_user.icu_athlete_id,
            db_user.access_token as _
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn user_update_token(
        &mut self,
        owner: IcuId,
        token: Option<String>,
    ) -> TbResult<IcuUser> {
        sqlx::query_as!(
            DbIcuUser,
            "UPDATE icu_users
             SET access_token = $2
             WHERE icu_athlete_id = $1
             RETURNING *",
            owner.to_string(),
            token
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn user_delete(&mut self, user: UserId) -> TbResult<usize> {
        let result = sqlx::query!("DELETE FROM icu_users WHERE tbid = $1", i32::from(user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    /// The disable policy (a 401's policy): drop the user's queued events,
    /// then clear their (durable) token (the cleared token is the disable
    /// signal).
    async fn user_disable(&mut self, owner: IcuId) -> TbResult<()> {
        let _ = sqlx::query!(
            "DELETE FROM icu_events WHERE icu_athlete_id = $1",
            owner.to_string()
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        tb_provider::Store::<tb_provider::icu::IcuSpec>::user_update_token(self, owner, None)
            .await?;
        Ok(())
    }

    async fn user_event_count(&mut self, owner: &IcuId) -> TbResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM icu_events WHERE icu_athlete_id = $1",
            owner.to_string()
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(count.unwrap_or(0))
    }

    async fn event_get_next(&mut self, owner: IcuId) -> TbResult<Option<IcuEvent>> {
        sqlx::query_as!(
            DbIcuEvent,
            r#"SELECT id, icu_athlete_id, activity_id as "activity_id!", event_time, object_type as "object_type!", event_type as "event_type!", oldest FROM icu_events
             WHERE icu_athlete_id = $1
             ORDER BY event_time ASC
             LIMIT 1"#,
            owner.to_string()
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn event_get_later(&mut self, key: String, owner: IcuId) -> TbResult<Vec<IcuEvent>> {
        sqlx::query_as!(
            DbIcuEvent,
            r#"SELECT id, icu_athlete_id, activity_id as "activity_id!", event_time, object_type as "object_type!", event_type as "event_type!", oldest FROM icu_events
               WHERE activity_id = $1 AND icu_athlete_id = $2
               ORDER BY event_time ASC"#,
            key,
            owner.to_string()
        )
        .fetch_all(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(vec_into)
    }

    async fn event_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        sqlx::query!("DELETE FROM icu_events WHERE id = ANY($1)", values as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        sqlx::query!("DELETE FROM icu_events WHERE id = $1", event_id)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn event_set_time(&mut self, event_id: Option<i32>, time: i64) -> TbResult<()> {
        sqlx::query!(
            "UPDATE icu_events SET event_time = $2 WHERE id = $1",
            event_id,
            time
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn event_delete_sync(&mut self, owner: &IcuId) -> TbResult<usize> {
        let result = sqlx::query!(
            "DELETE FROM icu_events WHERE icu_athlete_id = $1 AND object_type = 'SYNC'",
            owner.to_string()
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        let e: DbIcuEvent = e.into();
        sqlx::query_as!(
            DbIcuEvent,
            r#"INSERT INTO icu_events (icu_athlete_id, activity_id, event_time, object_type, event_type, oldest)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, icu_athlete_id, activity_id as "activity_id!", event_time, object_type as "object_type!", event_type as "event_type!", oldest"#,
            e.icu_athlete_id,
            e.activity_id,
            e.event_time,
            e.object_type,
            e.event_type,
            e.oldest
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    fn limited_until(&self) -> Option<i64> {
        tb_provider::icu::limited_until()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tb_provider::icu::IcuObjectType;

    fn icu_user(token: Option<&str>) -> IcuUser {
        IcuUser {
            id: IcuId::from("ath-42"),
            tendabike_id: UserId::from(1),
            access_token: token.map(|t| AccessToken::new(t.to_string())),
        }
    }

    #[test]
    fn icu_user_roundtrips() {
        let user = icu_user(Some("durable-token"));
        let db = DbIcuUser::from(user.clone());
        assert_eq!(db.tbid, 1);
        assert_eq!(db.icu_athlete_id, "ath-42");
        assert_eq!(db.access_token.as_deref(), Some("durable-token"));
        let back = IcuUser::from(db);
        assert_eq!(back.id, IcuId::from("ath-42"));
        assert_eq!(back.tendabike_id, UserId::from(1));
        assert_eq!(
            back.access_token.as_ref().map(|t| t.secret().to_string()),
            Some("durable-token".to_string())
        );
    }

    #[test]
    fn icu_event_roundtrips() {
        // An `ACTIVITY` event: `oldest` is `None` (no walk).
        let event = IcuEvent {
            id: Some(7),
            icu_athlete_id: IcuId::from("ath-42"),
            activity_id: "1234".to_string(),
            event_time: 1700000000,
            object_type: IcuObjectType::Activity,
            event_type: "ACTIVITY_DELETED".to_string(),
            oldest: None,
        };
        let db = DbIcuEvent::from(event.clone());
        assert_eq!(db.icu_athlete_id, "ath-42");
        assert_eq!(db.activity_id, "1234");
        assert_eq!(db.event_time, 1700000000);
        assert_eq!(db.object_type, "ACTIVITY");
        assert_eq!(db.event_type, "ACTIVITY_DELETED");
        assert_eq!(db.oldest, None);
        let back = IcuEvent::from(db);
        assert_eq!(back.activity_id, "1234");
        assert_eq!(back.object_type, IcuObjectType::Activity);
        assert_eq!(back.event_type, "ACTIVITY_DELETED");
        assert_eq!(back.oldest, None);

        // A `SYNC` event: `oldest` is the walk's lower bound.
        let sync = IcuEvent {
            id: Some(8),
            icu_athlete_id: IcuId::from("ath-42"),
            activity_id: "sync".to_string(),
            event_time: 0,
            object_type: IcuObjectType::Sync,
            event_type: String::new(),
            oldest: Some(1_700_000_000),
        };
        let db = DbIcuEvent::from(sync.clone());
        assert_eq!(db.object_type, "SYNC");
        assert_eq!(db.oldest, Some(1_700_000_000));
        let back = IcuEvent::from(db);
        assert_eq!(back.oldest, Some(1_700_000_000));
        assert_eq!(back.object_type, IcuObjectType::Sync);
    }
}
