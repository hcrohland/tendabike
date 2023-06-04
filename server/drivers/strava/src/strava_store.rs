use crate::event::Event;
use crate::{ActivityId, NewPart, PartId, Person, StravaId, StravaUser, UserId};
use s_diesel::{schema, AppConn};
use anyhow::Context;
use anyhow::Result as AnyResult;
use diesel::prelude::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
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

    strava_activities
        .find(strava_id)
        .select(tendabike_id)
        .for_update()
        .get_result::<ActivityId>(conn)
        .await
        .optional()
        .context("failed to get tbid for stravaid")
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

pub async fn get_stravaid_for_tb_activity(
    act: i32,
    conn: &mut AppConn,
) -> Result<i64, anyhow::Error> {
    use schema::strava_activities::dsl::*;
    strava_activities
        .filter(tendabike_id.eq(act))
        .select(id)
        .first(conn)
        .await
        .context("failed to get stravaid for activity")
}

pub async fn delete_strava_activity(
    act_id: i64,
    conn: &mut AppConn,
) -> Result<usize, anyhow::Error> {
    use schema::strava_activities::dsl::*;
    diesel::delete(strava_activities.find(act_id))
        .execute(conn)
        .await
        .context("failed to delete strava activity")
}

pub async fn get_activityid_from_strava_activity(
    act_id: i64,
    conn: &mut AppConn,
) -> Result<Option<ActivityId>, anyhow::Error> {
    use schema::strava_activities::dsl::*;
    strava_activities
        .select(tendabike_id)
        .find(act_id)
        .first(conn)
        .await
        .optional()
        .context("failed to get activity id")
}

pub async fn create_new_gear(
    conn: &mut AppConn,
    strava_id: String,
    part: NewPart,
    user: &dyn Person,
) -> Result<PartId, anyhow::Error> {
    conn.transaction(|conn| {
        async {
            use schema::strava_gears::dsl::*;
            // maybe the gear was created by now?
            if let Some(gear) = get_tbid_for_strava_gear(&strava_id, conn).await? {
                return Ok(gear);
            }

            let tbid = part.create(user, conn).await?.id;

            diesel::insert_into(strava_gears)
                .values((
                    id.eq(strava_id),
                    tendabike_id.eq(tbid),
                    user_id.eq(user.get_id()),
                ))
                .execute(conn)
                .await
                .context("couldn't store gear")?;
            Ok(tbid)
        }
        .scope_boxed()
    })
    .await
}

pub(crate) async fn delete_strava_event(
    event_id: Option<i32>,
    conn: &mut AppConn,
) -> Result<(), anyhow::Error> {
    use schema::strava_events::dsl::*;
    diesel::delete(strava_events)
        .filter(id.eq(event_id))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn set_event_time(
    e_id: Option<i32>,
    e_time: i64,
    conn: &mut AppConn,
) -> Result<(), anyhow::Error> {
    use schema::strava_events::dsl::*;
    diesel::update(strava_events)
        .filter(id.eq(e_id))
        .set(event_time.eq(e_time))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn store_event(e: Event, conn: &mut AppConn) -> anyhow::Result<()> {
    diesel::insert_into(schema::strava_events::table)
        .values(&e)
        .get_result::<Event>(conn)
        .await?;
    Ok(())
}

pub async fn get_next_event_for_stravauser(
    user: &crate::StravaUser,
    conn: &mut AppConn,
) -> AnyResult<Option<Event>> {
    use schema::strava_events::dsl::*;
    strava_events
        .filter(owner_id.eq_any(vec![0, user.id.into()]))
        .first::<Event>(conn)
        .await
        .optional()
        .context("failed to get next event")
}

pub async fn get_all_later_events_for_object(
    obj_id: i64,
    oid: StravaId,
    conn: &mut AppConn,
) -> AnyResult<Vec<Event>> {
    use schema::strava_events::dsl::*;
    strava_events
        .filter(object_id.eq(obj_id))
        .filter(owner_id.eq(oid))
        .order(event_time.asc())
        .get_results::<Event>(conn)
        .await
        .context("failed to read list of events")
}

pub async fn delete_events_by_vec_id(
    values: Vec<Option<i32>>,
    conn: &mut AppConn,
) -> AnyResult<()> {
    use schema::strava_events::dsl::*;

    diesel::delete(strava_events)
        .filter(id.eq_any(values))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn get_all_stravausers(conn: &mut AppConn) -> AnyResult<Vec<StravaUser>> {
    schema::strava_users::table
        .get_results::<StravaUser>(conn)
        .await
        .context("get_stats: could not read users".to_string())
}

pub async fn read_stravauser_for_userid(
    id: UserId,
    conn: &mut AppConn,
) -> Result<StravaUser, anyhow::Error> {
    schema::strava_users::table
        .filter(schema::strava_users::tendabike_id.eq(id))
        .get_result(conn)
        .await
        .context(format!("User::get: user {} not registered", id))
}

pub async fn read_stravauser_for_stravaid(
    id: StravaId,
    conn: &mut AppConn,
) -> Result<Vec<StravaUser>, anyhow::Error> {
    schema::strava_users::table
        .find(id)
        .get_results::<StravaUser>(conn)
        .await
        .context("failed to read stravauser")
}

pub async fn insert_stravauser(
    user: StravaUser,
    conn: &mut AppConn,
) -> Result<StravaUser, anyhow::Error> {
    diesel::insert_into(schema::strava_users::table)
        .values(&user)
        .get_result(conn)
        .await
        .context("failed to insert user")
}

pub async fn stravauser_update_last_activity(
    user: &StravaUser,
    time: i64,
    conn: &mut AppConn,
) -> Result<(), anyhow::Error> {
    use schema::strava_users::dsl::*;
    diesel::update(strava_users.find(user.id))
        .set(last_activity.eq(time))
        .execute(conn)
        .await
        .context("Could not update last_activity")?;
    Ok(())
}

pub async fn stravaid_update_token(
    stravaid: StravaId,
    access: &str,
    exp: i64,
    refresh: Option<&str>,
    conn: &mut AppConn,
) -> Result<StravaUser, anyhow::Error> {
    use schema::strava_users::dsl::*;
    diesel::update(strava_users.find(stravaid))
        .set((
            access_token.eq(access),
            expires_at.eq(exp),
            refresh_token.eq(refresh.unwrap()),
        ))
        .get_result(conn)
        .await
        .context("Could not store user")
}

/// return the open events and the disabled status for a user.
///
/// # Errors
///
/// This function will return an error if the database connection fails.
pub async fn get_count_of_events_for_user(user: &StravaUser, conn: &mut AppConn) -> AnyResult<i64> {
    use schema::strava_events::dsl::*;

    strava_events
        .count()
        .filter(owner_id.eq(user.id))
        .first(conn)
        .await
        .context("could not read strava events")
}

/// Disable the user data in the database by erasing the access token
pub async fn disable_stravauser(user: &StravaId, conn: &mut AppConn) -> AnyResult<()> {
    use schema::strava_users::dsl::*;
    diesel::update(strava_users.find(user))
        .set((expires_at.eq(0), access_token.eq("")))
        .execute(conn)
        .await
        .context(format!("Could not disable record for user {}", user))?;
    Ok(())
}


