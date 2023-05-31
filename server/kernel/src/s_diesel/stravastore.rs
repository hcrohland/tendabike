use crate::{
    domain::{ActivityId, PartId, UserId},
    s_diesel::{schema, AppConn},
};
use anyhow::Context;
use anyhow::Result as AnyResult;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use schema::strava_gears;
use serde_derive::{Deserialize, Serialize};

pub async fn get_user_id_from_strava_id(conn: &mut AppConn, who: i32) -> AnyResult<i32> {
    use schema::strava_users::dsl::*;
    let user_id: i32 = strava_users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(conn)
        .await?;
    Ok(user_id)
}

#[derive(Serialize, Deserialize, Debug, Queryable, Insertable)]
#[diesel(table_name = strava_gears)]
struct Gear {
    id: String,
    tendabike_id: i32,
    user_id: i32,
}

pub async fn get_tbid_for_strava_gear(
    strava_id: &str,
    conn: &mut AppConn,
) -> AnyResult<Option<PartId>> {
    use schema::strava_gears::dsl::*;

    strava_gears
        .find(strava_id)
        .select(tendabike_id)
        .for_update()
        .first(conn)
        .await
        .optional()
        .context("Error reading database")
}

pub async fn get_strava_name_for_gear_id(
    gear: i32,
    conn: &mut AppConn,
) -> Result<String, anyhow::Error> {
    use schema::strava_gears::dsl::*;
    let g: String = strava_gears
        .filter(tendabike_id.eq(gear))
        .select(id)
        .first(conn)
        .await?;
    Ok(g)
}

pub async fn get_tbid_for_strava_activity(
    strava_id: i64,
    conn: &mut AppConn,
) -> AnyResult<Option<ActivityId>> {
    use schema::strava_activities::dsl::*;

    let tb_id = strava_activities
        .find(strava_id)
        .select(tendabike_id)
        .for_update()
        .get_result::<ActivityId>(conn)
        .await
        .optional()?;
    Ok(tb_id)
}

pub async fn insert_new_activity(
    strava_id: i64,
    uid: UserId,
    new_id: ActivityId,
    conn: &mut AppConn,
) -> AnyResult<()> {
    use schema::strava_activities::dsl::*;

    diesel::insert_into(strava_activities)
        .values((id.eq(strava_id), tendabike_id.eq(new_id), user_id.eq(uid)))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_stravaid_for_tb_activity(act: i32, conn: &mut AppConn) -> Result<i64, anyhow::Error> {
    use schema::strava_activities::dsl::*;
    let g: i64 = strava_activities
        .filter(tendabike_id.eq(act))
        .select(id)
        .first(conn)
        .await?;
    Ok(g)
}
