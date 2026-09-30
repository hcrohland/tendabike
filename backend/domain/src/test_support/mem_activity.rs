use super::*;
use crate::{ActTypeId, Activity, ActivityId, PartId, TbResult, UserId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl ActivityStore for MemStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        self.activities.push(act.clone());
        Ok(act)
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        Ok(self.activities.iter().find(|a| a.id == aid).cloned())
    }

    async fn activity_update(&mut self, new: Activity) -> TbResult<Activity> {
        // One rule on both stores: the data fields are replaced, but the row
        // keeps its utc_offset (the new start is expressed in the stored
        // offset), device_name, and external_id; a missing row is NotFound.
        let pos = self
            .activities
            .iter()
            .position(|a| a.id == new.id)
            .ok_or(crate::Error::NotFound("activity not found".to_string()))?;
        let act = &mut self.activities[pos];
        let offset = act.start.offset();
        act.user_id = new.user_id;
        act.what = new.what;
        act.name = new.name;
        act.start = new.start.to_offset(offset);
        act.duration = new.duration;
        act.time = new.time;
        act.distance = new.distance;
        act.climb = new.climb;
        act.descend = new.descend;
        act.energy = new.energy;
        act.gear = new.gear;
        // device_name and external_id are preserved.
        Ok(act.clone())
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        let len_before = self.activities.len();
        self.activities.retain(|a| a.id != aid);
        Ok(len_before - self.activities.len())
    }

    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        let ids: Vec<ActivityId> = activities.iter().map(|a| a.id).collect();
        let len_before = self.activities.len();
        self.activities.retain(|a| !ids.contains(&a.id));
        Ok(len_before - self.activities.len())
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        Ok(self
            .activities
            .iter()
            .filter(|a| &a.user_id == uid)
            .cloned()
            .collect())
    }

    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        Ok(self
            .activities
            .iter()
            // One rule on both stores: begin is included, end is excluded.
            .filter(|a| a.gear == Some(part) && a.start >= begin && a.start < end)
            .cloned()
            .collect())
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        // One rule on both stores: the activity's local wall-clock minute
        // (its start in the stored offset, floored to the minute) must equal
        // the query's UTC wall-clock minute. Zero matches → NotFound; if
        // several activities share the minute, the first is returned (the
        // same as Postgres' fetch_one).
        let query_minute = minute_floor(rstart.unix_timestamp());
        self.activities
            .iter()
            .find(|a| {
                a.user_id == uid
                    && minute_floor(a.start.unix_timestamp())
                        + a.start.offset().whole_seconds() as i64
                        == query_minute
            })
            .cloned()
            .ok_or(crate::Error::NotFound("activity not found".to_string()))
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        let mut updated = Vec::new();
        for act in self.activities.iter_mut() {
            if act.user_id == user && act.gear.is_none() && types.contains(&act.what) {
                act.gear = Some(*partid);
                updated.push(act.clone());
            }
        }
        Ok(updated)
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        Ok(self.activities.clone())
    }
}

/// A unix timestamp floored to the 60-second minute boundary. Floor (not
/// truncate-toward-zero) so pre-1970 instants behave like the database's
/// `date_trunc('minute', …)` on timestamptz.
fn minute_floor(unix: i64) -> i64 {
    unix - unix.rem_euclid(60)
}
