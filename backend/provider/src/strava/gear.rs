//! This module contains the implementation of StravaGear, a struct that represents a gear object from Strava API.
//! It also contains functions to convert StravaGear to Tendabike's Part object and to map Strava gear_id to Tendabike gear_id.
//!

use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::http::request_json;
use super::{OnBehalf, PartId, PartTypeId, StravaSpec, TbResult, UserId};
use crate::EntityFacade;
use crate::store::Store;

#[derive(Serialize, Deserialize, Debug)]
struct StravaGear {
    id: String,
    name: String,
    brand_name: Option<String>,
    model_name: Option<String>,
    /// What kind of bike. Only bikes have a frametype.
    /// Id None it is shoes
    frame_type: Option<i32>,
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
pub(crate) async fn into_partid<D: EntityFacade<S>, S: Store<StravaSpec>>(
    strava_id: String,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<PartId> {
    if let Some(gear) = facade.part_by_source(&strava_id, store).await? {
        return Ok(gear.id);
    }

    debug!("New Gear");
    let gear = request_json::<StravaGear>(user_id, &format!("/gear/{}", strava_id), store)
        .await
        .context("Couldn't map gear")?;

    // maybe the gear was created by now?
    if let Some(gear) = facade.part_by_source(&strava_id, store).await? {
        return Ok(gear.id);
    }

    let what = gear.what();
    let source = Some(gear.id);
    let vendor = gear.brand_name.unwrap_or("".into());
    let model = gear.model_name.unwrap_or("".into());
    let name = gear.name;
    let purchase = OffsetDateTime::now_utc();
    let session = Box::new(OnBehalf::new(user_id));
    let tbid = facade
        .create_part(name, vendor, model, what, source, purchase, session, store)
        .await?
        .id;
    Ok(tbid)
}

#[cfg(test)]
mod gear_tests;
