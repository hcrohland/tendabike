use crate::{SqlxConn, into_domain, vec_into};
use anyhow::Context;
use sqlx::FromRow;
use tb_domain::{ActTypeId, Activity, ActivityId, PartId, TbResult, UserId};
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, FromRow, PartialEq)]
struct DbActivity {
    /// The athlete
    user_id: i32,
    /// The activity type
    what: i32,
    /// This name of the activity.
    name: String,
    /// Start time
    start: OffsetDateTime,
    /// End time
    duration: i32,
    /// activity time
    time: Option<i32>,
    /// Covered distance
    distance: Option<i32>,
    /// Total climbing
    climb: Option<i32>,
    /// Total descending
    descend: Option<i32>,
    /// average energy output
    energy: Option<i32>,
    /// Which gear did she use?
    gear: Option<i32>,
    /// utc offset since timstamptz does not store the timezone
    utc_offset: i32,
    /// The primary key
    id: i64,
    /// device name
    device_name: Option<String>,
    external_id: Option<String>,
}

impl From<Activity> for DbActivity {
    fn from(v: Activity) -> Self {
        let Activity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
            device_name,
            external_id,
        } = v;
        let utc_offset = start.offset().whole_seconds();
        DbActivity {
            id: id.into(),
            user_id: user_id.into(),
            what: what.into(),
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear: gear.map(Into::into),
            utc_offset,
            device_name,
            external_id,
        }
    }
}

impl TryFrom<DbActivity> for Activity {
    type Error = tb_domain::Error;

    fn try_from(v: DbActivity) -> Result<Self, Self::Error> {
        let DbActivity {
            id,
            user_id,
            what,
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear,
            utc_offset,
            device_name,
            external_id,
        } = v;
        let utc_offset = ((utc_offset + 900) / 1800) * 1800; //round it to 1800s
        let offset = UtcOffset::from_whole_seconds(utc_offset).context("Utc Offset invalid")?;
        let start = start.to_offset(offset);

        Ok(Activity {
            id: id.into(),
            user_id: user_id.into(),
            what: what.into(),
            name,
            start,
            duration,
            time,
            distance,
            climb,
            descend,
            energy,
            gear: gear.map(Into::into),
            device_name,
            external_id,
        })
    }
}

fn vec_tryinto(db: Result<Vec<DbActivity>, sqlx::Error>) -> TbResult<Vec<Activity>> {
    db.map_err(into_domain)?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
}

#[async_trait::async_trait]
impl<'c> tb_domain::ActivityStore for SqlxConn<'c> {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        let values = DbActivity::from(act);
        sqlx::query_as!(
            DbActivity,
            "INSERT INTO activities (id, user_id, what, name, start, duration, time, distance, climb, descend, energy, gear, utc_offset, device_name, external_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING *",
            values.id,
            values.user_id,
            values.what,
            values.name,
            values.start,
            values.duration,
            values.time,
            values.distance,
            values.climb,
            values.descend,
            values.energy,
            values.gear,
            values.utc_offset,
            values.device_name,
            values.external_id
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        sqlx::query_as!(
            DbActivity,
            "SELECT * FROM activities WHERE id = $1 FOR UPDATE",
            i64::from(aid)
        )
        .fetch_optional(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .map(TryInto::try_into)
        .transpose()
    }

    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        let act = DbActivity::from(act);

        // do not update the offset, device_name and external_id. They might be lost in the frontend
        sqlx::query_as!(
            DbActivity,
            "UPDATE activities
             SET user_id = $2, what = $3, name = $4, start = $5, duration = $6, time = $7,
                 distance = $8, climb = $9, descend = $10, energy = $11, gear = $12
             WHERE id = $1
             RETURNING *",
            act.id,
            act.user_id,
            act.what,
            act.name,
            act.start,
            act.duration,
            act.time,
            act.distance,
            act.climb,
            act.descend,
            act.energy,
            act.gear,
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        let result = sqlx::query!("DELETE FROM activities WHERE id = $1", i64::from(aid))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        vec_tryinto(
            sqlx::query_as!(
                DbActivity,
                "SELECT * FROM activities WHERE user_id = $1 ORDER BY start",
                i32::from(*uid)
            )
            .fetch_all(&mut **self.inner())
            .await,
        )
    }

    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        vec_tryinto(
            sqlx::query_as!(
                DbActivity,
                "SELECT * FROM activities WHERE gear = $1 AND start >= $2 AND start < $3",
                i32::from(part),
                begin,
                end
            )
            .fetch_all(&mut **self.inner())
            .await,
        )
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        sqlx::query_as!(
            DbActivity,
            "SELECT * FROM activities
             WHERE user_id = $1
               AND date_trunc('minute', start) + make_interval(0,0,0,0,0,0,utc_offset) = date_trunc('minute', $2::timestamptz)
             FOR UPDATE",
            i32::from(uid),
            rstart
        )
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        let types: Vec<i32> = vec_into(types);
        vec_tryinto(
            sqlx::query_as!(
                DbActivity,
                "UPDATE activities
                 SET gear = $3
                 WHERE user_id = $1 AND gear IS NULL AND what = ANY($2)
                 RETURNING *",
                i32::from(user),
                &types,
                i32::from(*partid)
            )
            .fetch_all(&mut **self.inner())
            .await,
        )
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        vec_tryinto(
            sqlx::query_as!(DbActivity, "SELECT * FROM activities ORDER BY id")
                .fetch_all(&mut **self.inner())
                .await,
        )
    }

    async fn activities_delete(&mut self, list: &[Activity]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| i64::from(s.id)).collect();

        let result = sqlx::query!("DELETE FROM activities WHERE id = ANY($1)", &list as _)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tb_domain::Error;

    fn instant(offset_secs: i32) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1718430600)
            .unwrap()
            .to_offset(UtcOffset::from_whole_seconds(offset_secs).unwrap())
    }

    fn activity() -> Activity {
        Activity {
            id: ActivityId::from(42),
            user_id: UserId::from(1),
            what: ActTypeId::from(4),
            name: "Morning Ride".to_string(),
            start: instant(7200),
            duration: 3600,
            time: Some(3600),
            distance: Some(25000),
            climb: Some(300),
            descend: Some(300),
            energy: Some(3000000),
            gear: Some(PartId::from(1)),
            device_name: Some("Garmin".to_string()),
            external_id: None,
        }
    }

    fn db_activity(utc_offset: i32) -> DbActivity {
        DbActivity {
            user_id: 1,
            what: 4,
            name: "Morning Ride".to_string(),
            start: instant(7200),
            duration: 3600,
            time: Some(3600),
            distance: Some(25000),
            climb: Some(300),
            descend: Some(300),
            energy: Some(3000000),
            gear: Some(1),
            utc_offset,
            id: 42,
            device_name: Some("Garmin".to_string()),
            external_id: None,
        }
    }

    #[test]
    fn activity_into_db_derives_utc_offset() {
        let db = DbActivity::from(activity());
        assert_eq!(db.utc_offset, 7200);
        assert_eq!(db.id, 42);
        assert_eq!(db.user_id, 1);
        assert_eq!(db.gear, Some(1));
    }

    #[test]
    fn activity_db_roundtrip_preserves_fields() {
        let activity = activity();
        let db = DbActivity::from(activity.clone());
        let back = Activity::try_from(db).unwrap();
        assert_eq!(back, activity);
        assert_eq!(back.start.offset().whole_seconds(), 7200);
    }

    #[test]
    fn activity_db_roundtrip_rounds_offset_to_half_hours() {
        // ((offset + 900) / 1800) * 1800 truncates toward zero, so negative offsets shift toward zero
        for (stored, expected) in [(3601, 3600), (0, 0), (-3601, -1800), (-7200, -5400)] {
            let back = Activity::try_from(db_activity(stored)).unwrap();
            assert_eq!(back.start.offset().whole_seconds(), expected);
        }
    }

    #[test]
    fn activity_db_roundtrip_rejects_out_of_range_offset() {
        for offset in [100000, -100000] {
            let err = Activity::try_from(db_activity(offset)).unwrap_err();
            assert!(
                matches!(err, Error::AnyFailure(failure) if failure.to_string().contains("Utc Offset invalid"))
            );
        }
    }

    #[test]
    fn vec_tryinto_maps_all_rows() {
        let acts = vec_tryinto(Ok(vec![db_activity(7200), db_activity(0)])).unwrap();
        assert_eq!(acts.len(), 2);
        assert_eq!(acts[0].id, ActivityId::from(42));
    }

    #[test]
    fn vec_tryinto_propagates_domain_errors() {
        let err = vec_tryinto(Err(sqlx::Error::RowNotFound)).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }
}
