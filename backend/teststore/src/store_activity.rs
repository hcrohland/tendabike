use async_trait::async_trait;
use tb_domain::{
    ActTypeId, Activity, ActivityId, ActivityStore, PartId, TbResult, UserId, round_offset,
};
use time::{OffsetDateTime, UtcOffset};

use super::TestStore;

/// The production store keeps the start instant in a `timestamptz` and the
/// offset in a separate column; on every read it rounds the stored offset
/// to the nearest 30 minutes (truncation toward zero) and re-expresses the
/// instant in that offset — the instant never moves, only the label does.
/// This in-memory mirror applies the same rule to every activity it
/// returns (issue #409). See also the note on [`Activity`].
fn normalize_offset(a: &Activity) -> TbResult<Activity> {
    let rounded = round_offset(a.start.offset().whole_seconds());
    let offset = UtcOffset::from_whole_seconds(rounded)
        .map_err(|e| anyhow::anyhow!("Utc Offset invalid: {e}"))
        .map_err(tb_domain::Error::from)?;
    let mut a = a.clone();
    a.start = a.start.to_offset(offset);
    Ok(a)
}

#[async_trait]
impl ActivityStore for TestStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        // One rule on both stores (issue #405): the activity id is a
        // primary key — the database's INSERT hits the key on a duplicate,
        // and this in-memory mirror rejects the same insert instead of
        // silently accepting the duplicate row.
        if d.activities.iter().any(|a| a.id == act.id) {
            return Err(tb_domain::Error::DatabaseFailure(anyhow::anyhow!(
                "activity {} already exists",
                act.id
            )));
        }
        // The row keeps the instant (and the raw offset label); the value
        // returned is what a database read would return: rounded offset.
        d.activities.push(act.clone());
        normalize_offset(&act)
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        d.activities
            .iter()
            .find(|a| a.id == aid)
            .map(normalize_offset)
            .transpose()
    }

    async fn activity_update(&mut self, new: Activity) -> TbResult<Activity> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        // One rule on both stores: the data fields are replaced, but the row
        // keeps its utc_offset (the new start is expressed in the stored
        // offset), device_name, and external_id; a missing row is NotFound.
        let pos = d
            .activities
            .iter()
            .position(|a| a.id == new.id)
            .ok_or(tb_domain::Error::NotFound("activity not found".to_string()))?;
        let act = &mut d.activities[pos];
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
        normalize_offset(act)
    }

    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let len_before = d.activities.len();
        d.activities.retain(|a| a.id != aid);
        Ok(len_before - d.activities.len())
    }

    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let ids: Vec<ActivityId> = activities.iter().map(|a| a.id).collect();
        let len_before = d.activities.len();
        d.activities.retain(|a| !ids.contains(&a.id));
        Ok(len_before - d.activities.len())
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        let mut result = Vec::new();
        for a in d.activities.iter().filter(|a| &a.user_id == uid) {
            result.push(normalize_offset(a)?);
        }
        // One rule on both stores (issue #405): the database's ORDER BY
        // start — ascending start instant. The instant never moves under
        // offset normalization, and the stable sort keeps creation order
        // for equal instants (any order is acceptable for ties).
        result.sort_by_key(|a| a.start.unix_timestamp());
        Ok(result)
    }

    async fn activities_find_by_gear_and_time(
        &mut self,
        part: PartId,
        begin: OffsetDateTime,
        end: OffsetDateTime,
    ) -> TbResult<Vec<Activity>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        let mut result = Vec::new();
        for a in &d.activities {
            // One rule on both stores: begin is included, end is excluded.
            if a.gear == Some(part) && a.start >= begin && a.start < end {
                result.push(normalize_offset(a)?);
            }
        }
        Ok(result)
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        // One rule on both stores (maintainer-confirmed, issue #408): the
        // activity's local wall-clock minute (its start in the stored offset,
        // floored to the minute) must equal the query's UTC wall-clock
        // minute. Zero matches → NotFound; exactly one → that activity;
        // two or more → Ambiguous, never a silent first-match.
        let query_minute = minute_floor(rstart.unix_timestamp());
        let matched: Vec<Activity> = d
            .activities
            .iter()
            .filter(|a| {
                a.user_id == uid
                    && minute_floor(a.start.unix_timestamp())
                        + a.start.offset().whole_seconds() as i64
                        == query_minute
            })
            .cloned()
            .collect();
        match matched.len() {
            0 => Err(tb_domain::Error::NotFound("activity not found".to_string())),
            1 => normalize_offset(&matched[0]),
            n => Err(tb_domain::Error::Ambiguous(format!(
                "user {uid} has {n} activities in the minute of {rstart}"
            ))),
        }
    }

    async fn activity_set_gear_if_null(
        &mut self,
        user: UserId,
        types: Vec<ActTypeId>,
        partid: &PartId,
    ) -> TbResult<Vec<Activity>> {
        let d = &mut self.state.lock().expect("test store state poisoned").domain;
        let mut updated = Vec::new();
        for act in d.activities.iter_mut() {
            if act.user_id == user && act.gear.is_none() && types.contains(&act.what) {
                act.gear = Some(*partid);
                updated.push(act.clone());
            }
        }
        let mut result = Vec::new();
        for a in updated {
            result.push(normalize_offset(&a)?);
        }
        Ok(result)
    }

    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> {
        let d = &self.state.lock().expect("test store state poisoned").domain;
        let mut result = Vec::new();
        for a in &d.activities {
            result.push(normalize_offset(a)?);
        }
        Ok(result)
    }
}

/// A unix timestamp floored to the 60-second minute boundary. Floor (not
/// truncate-toward-zero) so pre-1970 instants behave like the database's
/// `date_trunc('minute', …)` on timestamptz.
fn minute_floor(unix: i64) -> i64 {
    unix - unix.rem_euclid(60)
}
