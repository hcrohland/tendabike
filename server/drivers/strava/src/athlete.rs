use super::*;

#[derive(Debug, Deserialize)]
pub struct StravaAthlete {
firstname: String,
lastname: String,
id: i32
}

impl StravaAthlete {
/// Get the user data from the Strava OAuth callback
pub fn retrieve(self, conn: &AppConn) -> Result<StravaUser> {
    info!("got {:?}", self);

    let user = strava_users::table.find(self.id).get_result::<StravaUser>(conn).optional()?;
    if let Some(user) = user {
        return Ok(user);
    }

    // create user!
    let tendabike_id = crate::User::create(self.firstname, self.lastname, conn)?;

    let user = StravaUser {
        id: self.id,
        tendabike_id,
        ..Default::default()
    };

    let user: StravaUser = diesel::insert_into(strava_users::table)
        .values(&user)
        .get_result(conn)?;
    sync_users(Some(user.id), 0, conn)?;
    Ok(user)
}
}
