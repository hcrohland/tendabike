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
    disabled: bool,
}

impl From<IcuUser> for DbIcuUser {
    fn from(value: IcuUser) -> Self {
        let IcuUser {
            id,
            tendabike_id,
            access_token,
            disabled,
        } = value;
        Self {
            id: 0,
            tbid: tendabike_id.into(),
            icu_athlete_id: id.into(),
            access_token,
            disabled,
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
            disabled,
        } = value;
        Self {
            id: icu_athlete_id.into(),
            tendabike_id: tbid.into(),
            access_token,
            disabled,
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
}

impl From<IcuEvent> for DbIcuEvent {
    fn from(value: IcuEvent) -> Self {
        let IcuEvent {
            id,
            icu_athlete_id,
            activity_id,
            event_time,
            object_type,
        } = value;
        Self {
            id,
            icu_athlete_id: icu_athlete_id.into(),
            activity_id,
            event_time,
            object_type,
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
        } = value;
        Self {
            id,
            icu_athlete_id: icu_athlete_id.into(),
            activity_id,
            event_time,
            object_type,
        }
    }
}

#[async_trait::async_trait]
impl<'c> tb_provider::icu::IcuDataStore for SqlxConn<'c> {
    async fn icuser_get_all(&mut self) -> TbResult<Vec<IcuUser>> {
        sqlx::query_as!(DbIcuUser, "SELECT * FROM icu_users")
            .fetch_all(&mut **self.inner())
            .await
            .map_err(into_domain)
            .map(vec_into)
    }

    async fn icuser_get_by_tbid(&mut self, id: UserId) -> TbResult<IcuUser> {
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

    async fn icuser_get_by_icuid(&mut self, id: &IcuId) -> TbResult<Option<IcuUser>> {
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

    async fn icuser_create(&mut self, user: IcuUser) -> TbResult<IcuUser> {
        let db_user = DbIcuUser::from(user);
        sqlx::query_as!(
            DbIcuUser,
            "INSERT INTO icu_users (tbid, icu_athlete_id, access_token, disabled)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
            db_user.tbid,
            db_user.icu_athlete_id,
            db_user.access_token as _,
            db_user.disabled
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn icuser_update_token(
        &mut self,
        icuid: IcuId,
        access_token: Option<String>,
    ) -> TbResult<IcuUser> {
        sqlx::query_as!(
            DbIcuUser,
            "UPDATE icu_users
             SET access_token = $2
             WHERE icu_athlete_id = $1
             RETURNING *",
            icuid.to_string(),
            access_token
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(Into::into)
    }

    async fn icuser_delete(&mut self, user: UserId) -> TbResult<usize> {
        let result = sqlx::query!("DELETE FROM icu_users WHERE tbid = $1", i32::from(user))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn icu_event_create(&mut self, e: IcuEvent) -> TbResult<()> {
        let e: DbIcuEvent = e.into();
        sqlx::query_as!(
            DbIcuEvent,
            r#"INSERT INTO icu_events (icu_athlete_id, activity_id, event_time, object_type)
             VALUES ($1, $2, $3, $4)
             RETURNING id, icu_athlete_id, activity_id as "activity_id!", event_time, object_type as "object_type!""#,
            e.icu_athlete_id,
            e.activity_id,
            e.event_time,
            e.object_type
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn icu_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        sqlx::query!("DELETE FROM icu_events WHERE id = $1", event_id)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;
        Ok(())
    }

    async fn icu_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        sqlx::query!(
            "UPDATE icu_events SET event_time = $2 WHERE id = $1",
            e_id,
            e_time
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(())
    }

    async fn icu_event_get_next_for_user(&mut self, user: IcuId) -> TbResult<Option<IcuEvent>> {
        sqlx::query_as!(
            DbIcuEvent,
            r#"SELECT id, icu_athlete_id, activity_id as "activity_id!", event_time, object_type as "object_type!" FROM icu_events
             WHERE icu_athlete_id = $1
             ORDER BY event_time ASC
             LIMIT 1"#,
            user.to_string()
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)
        .map(option_into)
    }

    async fn icu_events_get_count_for_user(&mut self, user: &IcuId) -> TbResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM icu_events WHERE icu_athlete_id = $1",
            user.to_string()
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?;
        Ok(count.unwrap_or(0))
    }

    async fn icu_events_delete_for_user(&mut self, user: &IcuId) -> TbResult<usize> {
        let result = sqlx::query!(
            "DELETE FROM icu_events WHERE icu_athlete_id = $1",
            user.to_string()
        )
        .execute(&mut **self.inner())
        .await
        .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}

impl<'c> tb_provider::icu::IcuStore for SqlxConn<'c> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn icu_user(token: Option<&str>) -> IcuUser {
        IcuUser {
            id: IcuId::from("ath-42"),
            tendabike_id: UserId::from(1),
            access_token: token.map(|t| t.to_string()),
            disabled: false,
        }
    }

    #[test]
    fn icu_user_roundtrips() {
        let user = icu_user(Some("durable-token"));
        let db = DbIcuUser::from(user.clone());
        assert_eq!(db.tbid, 1);
        assert_eq!(db.icu_athlete_id, "ath-42");
        assert_eq!(db.access_token.as_deref(), Some("durable-token"));
        assert!(!db.disabled);
        let back = IcuUser::from(db);
        assert_eq!(back.id, IcuId::from("ath-42"));
        assert_eq!(back.tendabike_id, UserId::from(1));
        assert_eq!(back.access_token.as_deref(), Some("durable-token"));
    }

    #[test]
    fn icu_event_roundtrips() {
        let event = IcuEvent {
            id: Some(7),
            icu_athlete_id: IcuId::from("ath-42"),
            activity_id: "1234".to_string(),
            event_time: 1700000000,
            object_type: "ACTIVITY".to_string(),
        };
        let db = DbIcuEvent::from(event.clone());
        assert_eq!(db.icu_athlete_id, "ath-42");
        assert_eq!(db.activity_id, "1234");
        assert_eq!(db.event_time, 1700000000);
        assert_eq!(IcuEvent::from(db).activity_id, "1234");
    }
}
