//! This module contains the implementation of StravaGear, a struct that represents a gear object from Strava API.
//! It also contains functions to convert StravaGear to Tendabike's Part object and to map Strava gear_id to Tendabike gear_id.
//!

use time::OffsetDateTime;

use crate::*;

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

pub async fn strava_url(gear: i32, store: &mut impl StravaStore) -> TbResult<String> {
    let mut g = store.strava_gearid_get_name(gear).await?;
    if g.remove(0) != 'b' {
        return Err(Error::BadRequest(format!("'{g}' is not a bike id")));
    }

    Ok(format!("https://strava.com/bikes/{}", &g))
}

impl StravaGear {
    fn what(&self) -> PartTypeId {
        match self.frame_type {
            None => 301,  // shoes
            Some(_) => 1, // bikes
        }
        .into()
    }
}

/// map strava gear_id to tb gear_id
///
/// If it does not exist create it at tb
/// None will return None
pub(crate) async fn strava_to_tb(
    strava_id: String,
    user: &mut impl StravaPerson,
    store: &mut impl StravaStore,
) -> TbResult<PartId> {
    if let Some(gear) = store.strava_gear_get_tbid(&strava_id).await? {
        return Ok(gear);
    }

    debug!("New Gear");
    let gear = user
        .request_json::<StravaGear>(&format!("/gear/{}", &strava_id), store)
        .await
        .context("Couldn't map gear")?;

    store
        .transaction(|store| {
            async {
                // maybe the gear was created by now?
                if let Some(gear) = store.strava_gear_get_tbid(&strava_id).await? {
                    return Ok(gear);
                }

                let what = gear.what();
                let source = Some(gear.id);
                let vendor = gear.brand_name.unwrap_or("".into());
                let model = gear.model_name.unwrap_or("".into());
                let name = gear.name;
                let purchase = OffsetDateTime::now_utc();
                let tbid = Part::create(name, vendor, model, what, source, purchase, user, store)
                    .await?
                    .id;

                store
                    .strava_gear_new(strava_id, tbid, user.get_id())
                    .await?;
                Ok(tbid)
            }
            .scope_boxed()
        })
        .await
}
