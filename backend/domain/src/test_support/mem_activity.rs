use super::*;
use crate::{ActTypeId, Activity, ActivityId, PartId, TbResult, UserId};
use time::{OffsetDateTime, UtcOffset};

/// The production store keeps the start instant in a `timestamptz` and the
/// offset in a separate column; on every read it rounds the stored offset
/// to the nearest 30 minutes (truncation toward zero) and re-expresses the
/// instant in that offset — the instant never moves, only the label does.
/// This in-memory mirror applies the same rule to every activity it
/// returns (issue #409). See also the note on [`Activity`].
fn normalize_offset(a: &Activity) -> TbResult<Activity> {
    let rounded = ((a.start.offset().whole_seconds() + 900) / 1800) * 1800;
    let offset = UtcOffset::from_whole_seconds(rounded)
        .map_err(|e| anyhow::anyhow!("Utc Offset invalid: {e}"))
        .map_err(crate::Error::from)?;
    let mut a = a.clone();
    a.start = a.start.to_offset(offset);
    Ok(a)
}

#[async_trait::async_trait]
impl ActivityStore for MemStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> {
        let d = self.state_mut();
        // The row keeps the instant (and the raw offset label); the value
        // returned is what a database read would return: rounded offset.
        d.activities.push(act.clone());
        normalize_offset(&act)
    }

    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> {
        let d = self.state();
        d.activities
            .iter()
            .find(|a| a.id == aid)
            .map(normalize_offset)
            .transpose()
    }

    async fn activity_update(&mut self, new: Activity) -> TbResult<Activity> {
        let d = self.state_mut();
        // One rule on both stores: the data fields are replaced, but the row
        // keeps its utc_offset (the new start is expressed in the stored
        // offset), device_name, and external_id; a missing row is NotFound.
        let pos = d
            .activities
            .iter()
            .position(|a| a.id == new.id)
            .ok_or(crate::Error::NotFound("activity not found".to_string()))?;
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
        let d = self.state_mut();
        let len_before = d.activities.len();
        d.activities.retain(|a| a.id != aid);
        Ok(len_before - d.activities.len())
    }

    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> {
        let d = self.state_mut();
        let ids: Vec<ActivityId> = activities.iter().map(|a| a.id).collect();
        let len_before = d.activities.len();
        d.activities.retain(|a| !ids.contains(&a.id));
        Ok(len_before - d.activities.len())
    }

    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> {
        let d = self.state();
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
        let d = self.state();
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
        let d = self.state();
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
            0 => Err(crate::Error::NotFound("activity not found".to_string())),
            1 => normalize_offset(&matched[0]),
            n => Err(crate::Error::Ambiguous(format!(
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
        let d = self.state_mut();
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
        let d = self.state();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActTypeId, Activity, ActivityId, UserId};
    use time::{OffsetDateTime, UtcOffset};

    fn activity_at(offset_secs: i32) -> Activity {
        Activity {
            id: ActivityId::new(1),
            user_id: UserId::from(1),
            what: ActTypeId::from(1),
            name: "Ride".to_string(),
            start: OffsetDateTime::from_unix_timestamp(1718430600)
                .unwrap()
                .to_offset(UtcOffset::from_whole_seconds(offset_secs).unwrap()),
            duration: 3600,
            time: None,
            distance: None,
            climb: None,
            descend: None,
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        }
    }

    /// The production store keeps the start instant in a `timestamptz` and
    /// the offset in 30-minute buckets: every activity it returns has its
    /// offset rounded to the nearest 30 minutes. The in-memory store must
    /// apply the same rule (issue #409): a +5:45 offset comes back as
    /// +6:00, the instant unchanged.
    #[tokio::test]
    async fn activity_offsets_normalized_to_30_minutes() -> TbResult<()> {
        let mut store = MemStore::new();

        let created = store.activity_create(activity_at(20_700)).await?;
        assert_eq!(
            created.start.offset().whole_seconds(),
            21_600,
            "+5:45 must round to +6:00"
        );
        assert_eq!(
            created.start.unix_timestamp(),
            1718430600,
            "the instant must not move"
        );

        let read = store
            .activity_read_by_id(created.id)
            .await?
            .expect("created activity readable");
        assert_eq!(read.start.offset().whole_seconds(), 21_600);
        Ok(())
    }

    /// Negative offsets round toward zero the same way the database does:
    /// -5:45 becomes -5:30 (issue #409).
    #[tokio::test]
    async fn activity_negative_offsets_round_toward_zero() -> TbResult<()> {
        let mut store = MemStore::new();

        let created = store.activity_create(activity_at(-20_700)).await?;
        assert_eq!(
            created.start.offset().whole_seconds(),
            -19_800,
            "-5:45 must round to -5:30"
        );
        assert_eq!(created.start.unix_timestamp(), 1718430600);
        Ok(())
    }

    /// Offsets already on a 30-minute boundary are left untouched (issue
    /// #409).
    #[tokio::test]
    async fn activity_offsets_on_boundary_unchanged() -> TbResult<()> {
        let mut store = MemStore::new();

        let created = store.activity_create(activity_at(3_600)).await?; // +1:00
        assert_eq!(created.start.offset().whole_seconds(), 3_600);
        Ok(())
    }

    /// `get_all` returns the user's activities in ascending start instant —
    /// the database's `ORDER BY start` is the one rule on both stores
    /// (issue #405). The later ride is created first; the listing must not
    /// follow creation order.
    #[tokio::test]
    async fn activity_get_all_orders_by_start() -> TbResult<()> {
        let mut store = MemStore::new();

        // The later ride is created first.
        let mut later = activity_at(0);
        later.id = ActivityId::new(2);
        later.name = "Later Ride".to_string();
        store.activity_create(later).await?;

        // The earlier ride is created second.
        let mut earlier = activity_at(0);
        earlier.id = ActivityId::new(1);
        earlier.name = "Earlier Ride".to_string();
        earlier.start = earlier.start - time::Duration::hours(1);
        store.activity_create(earlier).await?;

        let acts = store.get_all(&UserId::from(1)).await?;
        let names: Vec<&str> = acts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Earlier Ride", "Later Ride"]);
        let ids: Vec<ActivityId> = acts.iter().map(|a| a.id).collect();
        assert_eq!(ids, vec![ActivityId::new(1), ActivityId::new(2)]);
        Ok(())
    }
}
