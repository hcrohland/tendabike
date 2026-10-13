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

pub async fn strava_url<
    St: Store + StravaStore<ConcreteDomain<St>>,
    S: Session + StravaSession<ConcreteDomain<St>>,
>(
    gear: i32,
    user: &mut S,
    store: &mut St,
) -> TbResult<String> {
    let part = PartId::from(gear).part(user, store).await?;
    let g = part.source.ok_or(Error::NotFound("".to_string()))?;
    match &g[0..1] {
        "b" => Ok(format!("https://strava.com/bikes/{}", &g[1..])),
        "g" => Ok("https://www.strava.com/settings/gear".into()),
        _ => Err(Error::NotFound("".to_string())),
    }
}

impl StravaGear {
    fn what<D: Domain>(&self) -> D::PartTypeId {
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
pub(crate) async fn into_partid<D: Domain, S: Session + StravaSession<D>>(
    strava_id: String,
    user: &mut S,
    store: &mut D::Store,
) -> Result<D::PartId, D::Error>
where
    D::Store: StravaStore<D>,
{
    if let Some(gear) = D::part_by_source(&strava_id, store).await? {
        return Ok(D::part_id(&gear));
    }

    debug!("New Gear");
    let gear = user
        .request_json::<StravaGear>(&format!("/gear/{}", strava_id), store)
        .await
        .map_err(|e| D::any_failure(anyhow::anyhow!("Couldn't map gear: {e}")))?;

    // maybe the gear was created by now?
    if let Some(gear) = D::part_by_source(&strava_id, store).await? {
        return Ok(D::part_id(&gear));
    }

    let what = gear.what::<D>();
    let source = Some(gear.id);
    let vendor = gear.brand_name.unwrap_or("".into());
    let model = gear.model_name.unwrap_or("".into());
    let name = gear.name;
    let purchase = OffsetDateTime::now_utc();
    let part = D::create_part(name, vendor, model, what, source, purchase, user, store).await?;
    Ok(D::part_id(&part))
}

#[cfg(test)]
mod gear_tests;
