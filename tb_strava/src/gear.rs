
use diesel::prelude::*;
use diesel::{
    self,
    QueryDsl,
    RunQueryDsl,
};

use schema::gears;

use crate::*;
use auth::User;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct TbGear {
    /// The owner
    pub owner: i32,
    /// The type of the part
    pub what: i32,
    /// This name of the part.
    pub name: String,
    /// The vendor name
    pub vendor: String,
    /// The model name
    pub model: String,
/*  
    /// purchase date
    pub purchase: DateTime<Utc>,
    /// usage time
   	pub time: i32,
    /// Usage distance
	pub distance: i32,
	/// Overall climbing
    pub climb: i32,
    /// Overall descending
	pub descend: i32,
    /// usage count
    pub count: i32,
 */
    }

#[derive(Serialize, Deserialize, Debug)]
#[derive(Queryable, Insertable)]
pub struct Gear {
        id: String,
        tendabike_id: i32,
        user_id: i32,
}


impl StravaGear {
    pub fn into_tb (self, user: &User) -> TbResult<TbGear> {
        Ok(TbGear {
            owner: user.id(),
            what: self.what(),
            name: self.name,
            vendor: self.brand_name.unwrap_or_else(|| String::from("")),
            model: self.model_name.unwrap_or_else(|| String::from("")),
        })
    }
    
    fn what(&self) -> i32 {
        match self.frame_type {
            None => 301, // shoes
            Some(_) => 1, // bikes
        }
    }

    fn request(id: &str, user: &User) -> TbResult<StravaGear> {
        let r = user.request(&format!("/gear/{}", id))?;
        let res: StravaGear = serde_json::from_str(&r).context("Did not receive StravaGear format")?;
        Ok(res)
    }
}

impl TbGear {
    fn send_to_tb (&self) -> TbResult<i32> {
        let client = reqwest::Client::new();

        let res: i32 = client.post(&format!("{}{}", TB_URI, "/part"))
            .header("x-user-id", self.owner)
            .json(self)
            .send().context("Cold not contact engine")?
            .error_for_status().context("Engine returned error")?
            .json().context("Could not parse result to integer")?;
        
        Ok(res)
    }
}


/// map strava gear_id to tb gear_id
/// 
/// If it does not exist create it at tb
/// None will return None
pub fn strava_to_tb(strava: String, user: &User) -> TbResult<i32> {
    use schema::gears::dsl::*;
 
    let g = gears.find(&strava).select(tendabike_id).get_results::<i32>(user.conn()).context("Error reading database")?;

    if !g.is_empty() { 
        return Ok(g[0]) 
    }

    dbg!("New Gear");
    let tbid = StravaGear::request(&strava, user).context("Couldn't map gear")?
                .into_tb(user).context("Could map gear to tendabike format")?
                .send_to_tb().context("Cound send gear to tb")?;
    diesel::insert_into(gears)
        .values((id.eq(strava),tendabike_id.eq(tbid),user_id.eq(user.id())))
        .execute(user.conn()).context("couldn't store gear")?;
    Ok(tbid)
}