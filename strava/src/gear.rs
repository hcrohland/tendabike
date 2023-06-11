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

pub async fn strava_url(gear: i32, conn: &mut impl StravaStore) -> AnyResult<String> {
    let mut g = conn.strava_gearid_get_name(gear).await?;
    if g.remove(0) != 'b' {
        bail!("Not found");
    }

    Ok(format!("https://strava.com/bikes/{}", &g))
}

impl StravaGear {
    fn into_tb(self, user: &StravaUser) -> AnyResult<NewPart> {
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
    user: &StravaUser,
    conn: &mut impl StravaStore,
) -> AnyResult<PartId> {
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

/// Get list of gear for user from Strava
pub(crate) async fn update_user(
    user: &StravaUser,
    conn: &mut impl StravaStore,
) -> AnyResult<Vec<PartId>> {
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

    let ath: Athlete = user.request_json("/athlete", conn).await?;

    let mut parts = Vec::new();
    for gear in ath.bikes.into_iter().chain(ath.shoes) {
        parts.push(gear::strava_to_tb(gear.id, user, conn).await?);
    }

    Ok(parts)
}
