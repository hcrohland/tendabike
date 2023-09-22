//! This module contains the implementation of StravaGear, a struct that represents a gear object from Strava API.
//! It also contains functions to convert StravaGear to Tendabike's Part object and to map Strava gear_id to Tendabike gear_id.
//!

use super::*;

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

pub async fn strava_url(gear: i32, conn: &mut impl StravaStore) -> TbResult<String> {
    let mut g = conn.strava_gearid_get_name(gear).await?;
    if g.remove(0) != 'b' {
        return Err(Error::BadRequest(format!("'{g}' is not a bike id")));
    }

    Ok(format!("https://strava.com/bikes/{}", &g))
}

impl StravaGear {
    fn into_tb(self, user: &impl StravaPerson) -> TbResult<NewPart> {
        Ok(NewPart {
            owner: user.tb_id(),
            what: self.what().into(),
            name: self.name,
            vendor: self.brand_name.unwrap_or_else(|| String::from("")),
            model: self.model_name.unwrap_or_else(|| String::from("")),
            purchase: None,
        })
    }

    fn what(&self) -> i32 {
        match self.frame_type {
            None => 301,  // shoes
            Some(_) => 1, // bikes
        }
    }
}

/// map strava gear_id to tb gear_id
///
/// If it does not exist create it at tb
/// None will return None
pub(crate) async fn strava_to_tb(
    strava_id: String,
    user: &mut impl StravaPerson,
    conn: &mut impl StravaStore,
) -> TbResult<PartId> {
    if let Some(gear) = conn.strava_gear_get_tbid(&strava_id).await? {
        return Ok(gear);
    }

    debug!("New Gear");
    let part = user
        .request_json::<StravaGear>(&format!("/gear/{}", &strava_id), conn)
        .await
        .context("Couldn't map gear")?
        .into_tb(user)?;

    conn.transaction(|c| {
        async {
            // maybe the gear was created by now?
            if let Some(gear) = c.strava_gear_get_tbid(&strava_id).await? {
                return Ok(gear);
            }

            let tbid = part.create(user, c).await?.id;

            c.strava_gear_new(strava_id, tbid, user.get_id()).await?;
            Ok(tbid)
        }
        .scope_boxed()
    })
    .await
}