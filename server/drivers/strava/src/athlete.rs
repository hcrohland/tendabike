use super::*;
/// The Athlete data we get from Strava
#[derive(Debug, Deserialize)]
pub struct StravaAthlete {
firstname: String,
lastname: String,
id: i32
}

impl StravaAthlete {
    /// Retrieve existing user data or create a new user
    pub fn retrieve(self, conn: &AppConn) -> Result<StravaUser> {
        info!("got {:?}", self);

        let user = strava_users::table.find(self.id).get_result::<StravaUser>(conn).optional()?;
        if let Some(user) = user {
            return Ok(user);
        }

        // create new user!
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

pub fn strava_url(strava_id: i32, conn: &AppConn) -> Result<String> {
    let user_id = s_diesel::get_user_id_from_strava_id(conn, strava_id)?;
    Ok(format!("https://strava.com/athletes/{}", &user_id))
}