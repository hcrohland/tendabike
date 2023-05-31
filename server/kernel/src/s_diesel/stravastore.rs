use crate::s_diesel::{schema, AppConn};
use anyhow::Result as AnyResult;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

pub async fn get_user_id_from_strava_id(conn: &mut AppConn, who: i32) -> AnyResult<i32> {
    use schema::strava_users::dsl::*;
    let user_id: i32 = strava_users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(conn)
        .await?;
    Ok(user_id)
}
