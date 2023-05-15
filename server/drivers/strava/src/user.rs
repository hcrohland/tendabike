use super::*;

const API: &str = "https://www.strava.com/api/v3";

#[derive(Queryable, Insertable, Identifiable, Debug, Default)]
#[table_name = "strava_users"]
pub struct StravaUser {
pub id: i32,
pub tendabike_id: i32,
pub last_activity: i64,
pub access_token: String,
pub expires_at: i64,
pub refresh_token: String,
}

impl StravaUser {
    pub fn is_valid (&self) -> bool {
        self.expires_at > get_time()
    }

    pub fn read (id: i32, conn: &AppConn) -> Result<Self> {
        Ok(strava_users::table
            .filter(strava_users::tendabike_id.eq(id))
            .get_result(conn)
            .context(format!("User::get: user {} not registered", id))?)
    }

    pub fn update_db(&self, conn: &AppConn) -> Result<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(self.id))
            .set((expires_at.eq(0), access_token.eq("")))
            .execute(conn).context(format!("Could not disable record for user {}",self.id))?;
        Ok(())
    }

    pub fn tb_id(&self) -> i32 {
        self.tendabike_id
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub(crate) fn update_last(&self, time: i64, conn: &AppConn) -> Result<i64> {
        if self.last_activity >= time {
            return Ok(self.last_activity);
        }
        use schema::strava_users::dsl::*;

        diesel::update(strava_users.find(self.id))
            .set(last_activity.eq(time))
            .execute(conn).context("Could not update last_activity")?;
        Ok(time)
    }

    pub fn update(self, access: &str, expires: Option<i64>, refresh: Option<&str>, conn: &AppConn) -> Result<Self> {
        use schema::strava_users::dsl::*;
        
        let iat = get_time();
        let exp = expires.unwrap() as i64 + iat - 300; // 5 Minutes buffer
        let user: StravaUser = diesel::update(strava_users.find(self.id()))
            .set((
                access_token.eq(access),
                expires_at.eq(exp),
                refresh_token.eq(refresh.unwrap()),
            ))
            .get_result(conn).context("Could not store user")?;
        
        Ok(user)
    }

    pub fn lock (&self, conn: &AppConn) -> Result<()> {
        use diesel::sql_types::Bool;
        #[derive(QueryableByName, Debug)]
        struct Lock {
            #[sql_type = "Bool"]
            #[column_name = "pg_try_advisory_lock"]
            lock: bool
        }

        ensure!(
            sql_query(format!("SELECT pg_try_advisory_lock({});", self.id)).get_result::<Lock>(conn)?.lock,
            Error::Conflict(format!("Two sessions for user {}", self.id))
        );
        Ok(())
    }

    pub fn unlock(&self, conn: &AppConn) -> Result<()> {
        sql_query(format!("SELECT pg_advisory_unlock({});", self.id)).execute(conn)?;
        Ok(())
    }        

    /// return the open events and the disabled status for a user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    pub fn get_stats(&self, conn: &AppConn) -> Result<(i64, bool)> {
        use schema::strava_events::dsl::*;

        let events = strava_events.count().filter(owner_id.eq(self.tendabike_id)).first(conn)?;
        return Ok((events, self.expires_at == 0))
    }

    pub(crate) fn request(&self, uri: &str, conn: &PgConnection) -> AnyResult<String> {
        Ok(self.get_strava(uri, conn)?
            .text().context("Could not get response body")?)
    }

    /// request information from the Strava API
    ///
    /// will return Error::TryAgain on certain error conditions
    /// will disable the User if Strava responds with NOT_AUTH
    fn get_strava(&self, uri: &str, conn: &AppConn) -> AnyResult<reqwest::blocking::Response> {
        use reqwest::StatusCode;
        let resp = reqwest::blocking::Client::new()
            .get(&format!("{}{}", API, uri))
            .bearer_auth(&self.access_token)
            .send().context("Could not reach strava")?;

        let status = resp.status();
        if status.is_success() { return Ok(resp) }

        match status {
            StatusCode::TOO_MANY_REQUESTS | 
            StatusCode::BAD_GATEWAY | 
            StatusCode::SERVICE_UNAVAILABLE | 
            StatusCode::GATEWAY_TIMEOUT => {
                bail!(Error::TryAgain(status.canonical_reason().unwrap()))
            },
            StatusCode::UNAUTHORIZED => {
                self.disable(conn)?;
                bail!(Error::NotAuth("Strava request authorization withdrawn".to_string()))
            },
            _ => bail!(Error::BadRequest(
                    format!("Strava request error: {}", status.canonical_reason().unwrap_or("Unknown status received"))
                ))
        }
    }

     
    /// disable a user 
    fn disable(&self, conn: &AppConn) -> AnyResult<()> {

        let id = self.id();
        info!("disabling user {}", id);
        event::insert_sync(id, time::get_time().sec, conn)
            .context(format!("Could insert sync for user: {:?}", id))?;
        self.update_db(conn)
    }

    /// disable a user per admin request
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist, is already disabled 
    /// or has open events and if strava or the database is not reachable.
    pub fn admin_disable(self, conn: &AppConn) -> AnyResult<()> {
    
        let (events, disabled) = self.get_stats(conn)?;

        if disabled { bail!(Error::BadRequest(String::from("user already disabled!"))) }
        if events > 0 { bail!(Error::BadRequest(String::from("user has open events!"))) }

        reqwest::blocking::Client::new()
            .post("https://www.strava.com/oauth/deauthorize")
            .query(&[("access_token" , &self.access_token)])
            .bearer_auth(&self.access_token)
            .send().context("Could not reach strava")?
            .error_for_status()?;

        warn!("User {} disabled by admin", self.tb_id());
        self.disable(conn)
    }
}

impl Person for StravaUser {
    fn get_id(&self) -> i32 {
        self.tendabike_id
    }
    fn is_admin(&self) -> bool {
        false
    }
}


#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
    disabled: bool,
}

pub fn get_all_stats(conn: &AppConn) -> Result<Vec<StravaStat>> {
    let users = strava_users::table
        .get_results::<StravaUser>(conn)
        .context(format!("get_stats: could not read users"))?;

    users.into_iter().map(|u| {
        let uid = u.tendabike_id;
        let stat = User::get_stat(uid, conn)?;
        let (events, disabled) = u.get_stats(conn)?;
        Ok(StravaStat {stat, events, disabled})
    }).collect()
}

/// Get the strava id for all users
pub fn sync_users (user_id: Option<i32>, time: i64, conn: &AppConn) -> Result<()> {
    use schema::strava_users::dsl::*;

    let users =
        match user_id {
            Some(user ) => strava_users.filter(tendabike_id.eq(user)).select(id).get_results(conn)?,
            None => strava_users.select(id).get_results(conn)?
        };
        for user_id in users {
            event::insert_sync(user_id, time, &conn)?;
        };
        Ok(())
}

pub fn user_summary(user: &StravaUser, conn: &AppConn) -> Result<Summary> {
    use crate::*;
    gear::update_user(user, conn)?;
    let parts = Part::get_all(user, conn)?;
    let attachments = Attachment::for_parts(&parts,&conn)?;
    let activities = Activity::get_all(user, conn)?;
    Ok(Summary::new(activities, parts,attachments))
}
