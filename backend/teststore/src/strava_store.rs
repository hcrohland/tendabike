//! The `StravaStore` impl on [`TestStore`] (spec #510), operating on
//! the shared Strava state.

use async_trait::async_trait;
use oauth2::RefreshToken;
use std::sync::atomic::Ordering;
use tb_domain::{Error, TbResult, UserId};
use tb_strava::event::Event;
use tb_strava::{StravaId, StravaStore, StravaUser};

use super::TestStore;

#[async_trait]
impl StravaStore for TestStore {
    async fn stravaid_get_user_id(&mut self, who: i32) -> TbResult<i32> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        st.users
            .values()
            .find(|u| u.tendabike_id == UserId::from(who))
            .map(|u| i32::from(u.id))
            .ok_or_else(|| Error::NotFound(format!("strava user {who} not found")))
    }

    async fn strava_event_delete(&mut self, event_id: Option<i32>) -> TbResult<()> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        st.events.retain(|e| e.id != event_id);
        Ok(())
    }

    async fn strava_event_set_time(&mut self, e_id: Option<i32>, e_time: i64) -> TbResult<()> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        for e in &mut st.events {
            if e.id == e_id {
                e.event_time = e_time;
            }
        }
        Ok(())
    }

    async fn stravaevent_store(&mut self, mut e: Event) -> TbResult<()> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        st.next_event_id += 1;
        e.id = Some(st.next_event_id);
        st.events.push(e);
        Ok(())
    }

    async fn strava_event_get_next_for_user(&mut self, user: StravaId) -> TbResult<Option<Event>> {
        // The stream tests' panic seam (spec #446 §4.6): when armed, the
        // next Strava-queue read panics, killing the executor task.
        if self.panic.swap(false, Ordering::SeqCst) {
            panic!("test: forced executor panic (spec §4.6)");
        }
        let st = &self.state.lock().expect("test store state poisoned").strava;
        Ok(st
            .events
            .iter()
            .filter(|e| e.owner_id == user || e.owner_id == StravaId::default())
            .min_by_key(|e| e.event_time)
            .cloned())
    }

    async fn strava_event_get_later(&mut self, obj_id: i64, oid: StravaId) -> TbResult<Vec<Event>> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        let mut res: Vec<Event> = st
            .events
            .iter()
            .filter(|e| e.object_id == obj_id && e.owner_id == oid)
            .cloned()
            .collect();
        res.sort_by_key(|e| e.event_time);
        Ok(res)
    }

    async fn strava_events_delete_batch(&mut self, values: Vec<Option<i32>>) -> TbResult<()> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        st.events.retain(|e| !values.contains(&e.id));
        Ok(())
    }

    async fn stravausers_get_all(&mut self) -> TbResult<Vec<StravaUser>> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        Ok(st.users.values().cloned().collect())
    }

    async fn stravauser_get_by_tbid(&mut self, id: UserId) -> TbResult<StravaUser> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        st.users
            .get(&id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("user {id} not registered with strava")))
    }

    async fn stravauser_get_by_stravaid(&mut self, id: &StravaId) -> TbResult<Option<StravaUser>> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        Ok(st.users.values().find(|u| u.id == *id).cloned())
    }

    async fn stravauser_new(&mut self, user: StravaUser) -> TbResult<StravaUser> {
        let res = user.clone();
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        st.users.insert(user.tendabike_id, user);
        Ok(res)
    }

    async fn stravaid_update_token(
        &mut self,
        stravaid: StravaId,
        refresh: Option<&String>,
    ) -> TbResult<StravaUser> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        let user = st
            .users
            .values_mut()
            .find(|u| u.id == stravaid)
            .ok_or_else(|| Error::NotFound(format!("strava user {stravaid} not found")))?;
        user.refresh_token = refresh.map(|r| RefreshToken::new(r.clone()));
        Ok(user.clone())
    }

    async fn strava_events_get_count_for_user(&mut self, user: &StravaId) -> TbResult<i64> {
        let st = &self.state.lock().expect("test store state poisoned").strava;
        Ok(st.events.iter().filter(|e| &e.owner_id == user).count() as i64)
    }

    async fn strava_events_delete_for_user(&mut self, user: &StravaId) -> TbResult<usize> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        let before = st.events.len();
        st.events.retain(|e| &e.owner_id != user);
        Ok(before - st.events.len())
    }

    async fn stravauser_delete(&mut self, user: UserId) -> TbResult<usize> {
        let st = &mut self.state.lock().expect("test store state poisoned").strava;
        Ok(st.users.remove(&user).is_some() as usize)
    }
}
