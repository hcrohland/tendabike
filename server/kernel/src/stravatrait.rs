pub trait StravaStore {
    fn get_user_id_from_strava_id(&self, who: i32) -> anyhow::Result<i32>;
}