use crate::{SqlxConn, into_domain, vec_into};
use anyhow::Context;
use sqlx::FromRow;
use tb_domain::{ActTypeId, Activity, ActivityId, PartId, Person, TbResult, UserId};
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

#[async_session::async_trait]
impl tb_domain::ActivityStore for SqlxConn {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        let values = DbActivity::from(act);
        sqlx::query_as::<_, DbActivity>(
            "INSERT INTO activities (user_id, what, name, start, duration, time, distance, climb, descend, energy, gear, utc_offset, device_name, external_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             RETURNING *"
        )
        .bind(values.user_id)
        .bind(values.what)
        .bind(values.name)
        .bind(values.start)
        .bind(values.duration)
        .bind(values.time)
        .bind(values.distance)
        .bind(values.climb)
        .bind(values.descend)
        .bind(values.energy)
        .bind(values.gear)
        .bind(values.utc_offset)
        .bind(values.device_name)
        .bind(values.external_id)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        sqlx::query_as::<_, DbActivity>("SELECT * FROM activities WHERE id = $1 FOR UPDATE")
            .bind(i64::from(aid))
            .fetch_optional(&mut **self.inner())
            .await
            .map_err(into_domain)?
            .map(TryInto::try_into)
            .transpose()
    }

    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        let act = DbActivity::from(act);
        sqlx::query_as::<_, DbActivity>(
            "UPDATE activities
             SET user_id = $2, what = $3, name = $4, start = $5, duration = $6, time = $7,
                 distance = $8, climb = $9, descend = $10, energy = $11, gear = $12,
                 utc_offset = $13, device_name = $14, external_id = $15
             WHERE id = $1
             RETURNING *",
        )
        .bind(act.id)
        .bind(act.user_id)
        .bind(act.what)
        .bind(act.name)
        .bind(act.start)
        .bind(act.duration)
        .bind(act.time)
        .bind(act.distance)
        .bind(act.climb)
        .bind(act.descend)
        .bind(act.energy)
        .bind(act.gear)
        .bind(act.utc_offset)
        .bind(act.device_name)
        .bind(act.external_id)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        let result = sqlx::query("DELETE FROM activities WHERE id = $1")
            .bind(i64::from(aid))
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        vec_tryinto(
            sqlx::query_as::<_, DbActivity>(
                "SELECT * FROM activities WHERE user_id = $1 ORDER BY start",
            )
            .bind(i32::from(*uid))
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
            sqlx::query_as::<_, DbActivity>(
                "SELECT * FROM activities WHERE gear = $1 AND start >= $2 AND start < $3",
            )
            .bind(i32::from(part))
            .bind(begin)
            .bind(end)
            .fetch_all(&mut **self.inner())
            .await,
        )
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        sqlx::query_as::<_, DbActivity>(
            "SELECT * FROM activities
             WHERE user_id = $1
               AND date_trunc('minute', start) + make_interval(0,0,0,0,0,0,utc_offset) = date_trunc('minute', $2)
             FOR UPDATE"
        )
        .bind(i32::from(uid))
        .bind(rstart)
        .fetch_one(&mut **self.inner())
        .await
        .map_err(into_domain)?
        .try_into()
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: &dyn Person,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        let types: Vec<i32> = vec_into(types);
        vec_tryinto(
            sqlx::query_as::<_, DbActivity>(
                "UPDATE activities
                 SET gear = $3
                 WHERE user_id = $1 AND gear IS NULL AND what = ANY($2)
                 RETURNING *",
            )
            .bind(i32::from(user.get_id()))
            .bind(&types)
            .bind(i32::from(*partid))
            .fetch_all(&mut **self.inner())
            .await,
        )
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        vec_tryinto(
            sqlx::query_as::<_, DbActivity>("SELECT * FROM activities ORDER BY id")
                .fetch_all(&mut **self.inner())
                .await,
        )
    }

    async fn activities_delete(&mut self, list: &[Activity]) -> TbResult<usize> {
        let list: Vec<_> = list.iter().map(|s| i64::from(s.id)).collect();

        let result = sqlx::query("DELETE FROM activities WHERE id = ANY($1)")
            .bind(&list)
            .execute(&mut **self.inner())
            .await
            .map_err(into_domain)?;

        Ok(result.rows_affected() as usize)
    }
}
