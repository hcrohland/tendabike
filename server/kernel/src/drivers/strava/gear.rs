use diesel::{self, QueryDsl, RunQueryDsl};

use super::*;
use part::NewPart;

use schema::strava_gears;

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

pub fn strava_url(gear: i32, context: &dyn StravaContext) -> TbResult<String> {
    use schema::strava_gears::dsl::*;

    let mut g: String = strava_gears
        .filter(tendabike_id.eq(gear))
        .select(id)
        .first(context.conn())?;
    if g.remove(0) != 'b' {
        bail!("Not found");
    }

    Ok(format!("https://strava.com/bikes/{}", &g))
}

impl StravaGear {
    fn into_tb(self, context: &dyn StravaContext) -> TbResult<NewPart> {
        let (user,_) = context.split();
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

    fn request(id: &str, context: &dyn StravaContext) -> TbResult<StravaGear> {
        let r = context.request(&format!("/gear/{}", id))?;
        let res: StravaGear =
            serde_json::from_str(&r).context(format!("Did not receive StravaGear format: {:?}", r))?;
        Ok(res)
    }
}

fn get_tbid(strava_id: &str, conn: &AppConn) -> TbResult<Option<PartId>> {
    use schema::strava_gears::dsl::*;
    
    Ok(strava_gears
        .find(strava_id)
        .select(tendabike_id)
        .for_update()
        .first(conn)
        .optional().context("Error reading database")? )
}

/// map strava gear_id to tb gear_id
///
/// If it does not exist create it at tb
/// None will return None
pub fn strava_to_tb(strava_id: String, context: &dyn StravaContext) -> TbResult<PartId> {
    let (user, conn) = context.split();
    
    if let Some(gear) = get_tbid(&strava_id, conn)? { 
        return Ok(gear) 
    }
    
    debug!("New Gear");
    let part = StravaGear::request(&strava_id, context)
        .context("Couldn't map gear")?
        .into_tb(context)?;

    conn.transaction(||{
        use schema::strava_gears::dsl::*;
        // maybe the gear was created by now?
        if let Some(gear) = get_tbid(&strava_id, conn)? { 
            return Ok(gear) 
        }

        let tbid = part.create(user, conn)?.id;
    
        diesel::insert_into(strava_gears)
            .values((id.eq(strava_id), tendabike_id.eq(tbid), user_id.eq(user.tb_id())))
            .execute(conn).context("couldn't store gear")?;
        Ok(tbid)
    })
}

/// Get list of gear for user from Strava
pub fn update_user(context: &dyn StravaContext) -> TbResult<Vec<PartId>> {
    #[derive(Deserialize, Debug)]
    struct Gear {
        id: String,
    }

    #[derive(Deserialize, Debug)]
    struct Athlete {
        // firstname: String,
        // lastname: String,
        bikes: Vec<Gear>,
        shoes: Vec<Gear>,
    }

    let r = context.request("/athlete")?;
    let ath: Athlete = serde_json::from_str(&r)?;
    let parts = ath.bikes.into_iter()
        .chain(ath.shoes)
        .map(|gear| gear::strava_to_tb(gear.id, context))
        .collect::<TbResult<_>>()?;
    Ok(parts)
}
