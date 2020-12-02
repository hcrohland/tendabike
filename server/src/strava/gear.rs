use diesel::{self, QueryDsl, RunQueryDsl};

use super::*;
use auth::User;
use schema::strava_gears;
pub use crate::part::NewPart;
use crate::part::PartId;

#[derive(Serialize, Deserialize, Debug)]
pub struct StravaGear {
    id: String,
    name: String,
    brand_name: Option<String>,
    model_name: Option<String>,
    /// What kind of bike. Only bikes have a frametype.
    /// Id None it is shoes
    frame_type: Option<i32>,
}
#[derive(Serialize, Deserialize, Debug, Queryable, Insertable)]
#[table_name = "strava_gears"]
pub struct Gear {
    id: String,
    tendabike_id: i32,
    user_id: i32,
}

pub fn strava_url(gear: i32, user: &User) -> TbResult<String> {
    use schema::strava_gears::dsl::*;

    let mut g: String = strava_gears
        .filter(tendabike_id.eq(gear))
        .select(id)
        .first(user.conn())?;
    if g.remove(0) != 'b' {
        bail!("Not found");
    }

    Ok(format!("https://strava.com/bikes/{}", &g))
}

impl StravaGear {
    pub fn into_tb(self, user: &User) -> TbResult<NewPart> {
        Ok(NewPart {
            owner: user.tb_id(),
            what: self.what().into(),
            name: self.name,
            vendor: self.brand_name.unwrap_or_else(|| String::from("")),
            model: self.model_name.unwrap_or_else(|| String::from("")),
            purchase: None
        })
    }

    fn what(&self) -> i32 {
        match self.frame_type {
            None => 301,  // shoes
            Some(_) => 1, // bikes
        }
    }

    fn request(id: &str, user: &User) -> TbResult<StravaGear> {
        let r = user.request(&format!("/gear/{}", id))?;
        let res: StravaGear =
            serde_json::from_str(&r).context(format!("Did not receive StravaGear format: {:?}", r))?;
        Ok(res)
    }
}

fn get_tbid(strava_id: &str, user: &User) -> TbResult<Option<PartId>> {
    use schema::strava_gears::dsl::*;
    
    Ok(strava_gears
        .find(strava_id)
        .select(tendabike_id)
        .for_update()
        .first(user.conn())
        .optional().context("Error reading database")? )
}

/// map strava gear_id to tb gear_id
///
/// If it does not exist create it at tb
/// None will return None
pub fn strava_to_tb(strava_id: String, user: &User) -> TbResult<PartId> {
    
    if let Some(g) = get_tbid(&strava_id, user)? { return Ok(g) }

    debug!("New Gear");
    let part = StravaGear::request(&strava_id, user)
        .context("Couldn't map gear")?
        .into_tb(user)?;

    user.conn().transaction(||{
        use schema::strava_gears::dsl::*;

        // maybe the gear was created by now?
        if let Some(g) = get_tbid(&strava_id, user)? { return Ok(g) }

        let tbid = part.create(user, user.conn())?.id;
    
        diesel::insert_into(strava_gears)
            .values((id.eq(strava_id), tendabike_id.eq(tbid), user_id.eq(user.tb_id())))
            .execute(user.conn()).context("couldn't store gear")?;
        Ok(tbid)
    })
}
