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

    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> {
        if let Some(pos) = self.activities.iter().position(|a| a.id == act.id) {
            self.activities[pos] = act.clone();
        }
        Ok(act)
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
            .filter(|a| a.gear == Some(part) && a.start >= begin && a.start <= end)
            .cloned()
            .collect())
    }

    async fn get_by_user_and_time(
        &mut self,
        uid: UserId,
        rstart: OffsetDateTime,
    ) -> TbResult<Activity> {
        self.activities
            .iter()
            .find(|a| a.user_id == uid && a.start == rstart)
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
