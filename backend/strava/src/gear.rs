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

pub async fn strava_url(
    gear: i32,
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
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
pub(crate) async fn into_partid(
    strava_id: String,
    user: &mut impl StravaSession,
    store: &mut impl StravaStore,
) -> TbResult<PartId> {
    if let Some(gear) = store.partid_get_by_source(&strava_id).await? {
        return Ok(gear);
    }

    debug!("New Gear");
    let gear = user
        .request_json::<StravaGear>(&format!("/gear/{}", strava_id), store)
        .await
        .context("Couldn't map gear")?;

    // maybe the gear was created by now?
    if let Some(gear) = store.partid_get_by_source(&strava_id).await? {
        return Ok(gear);
    }

    let what = gear.what();
    let source = Some(gear.id);
    let vendor = gear.brand_name.unwrap_or("".into());
    let model = gear.model_name.unwrap_or("".into());
    let name = gear.name;
    let purchase = OffsetDateTime::now_utc();
    let notes = String::new();
    let tbid = Part::create(
        name, vendor, model, what, source, purchase, notes, user, store,
    )
    .await?
    .id;
    Ok(tbid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TestStravaSession, TestStravaStore, gear_json, strava_user};

    fn setup() -> (TestStravaStore, TestStravaSession) {
        let mut store = TestStravaStore::new();
        store.insert_user(strava_user(UserId::from(1), 42, true));
        let session = TestStravaSession::new(UserId::from(1), 42.into());
        (store, session)
    }

    #[tokio::test]
    async fn into_partid_creates_bike() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/gear/b1", &gear_json("b1", Some(0)));
        let part_id = into_partid("b1".to_string(), &mut session, &mut store).await?;
        let part = PartStore::partid_get_part(&mut store.mem, part_id).await?;
        assert_eq!(part.what, 1.into());
        assert_eq!(part.source.as_deref(), Some("b1"));
        assert_eq!(part.vendor, "Test Brand");
        assert_eq!(part.model, "Test Model");
        assert_eq!(part.name, "Test Bike");
        Ok(())
    }

    #[tokio::test]
    async fn into_partid_creates_shoes() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue(
            "/gear/g1",
            r#"{"id":"g1","name":"Test Shoes","brand_name":null,"model_name":null,"frame_type":null}"#,
        );
        let part_id = into_partid("g1".to_string(), &mut session, &mut store).await?;
        let part = PartStore::partid_get_part(&mut store.mem, part_id).await?;
        assert_eq!(part.what, 301.into());
        assert_eq!(part.vendor, "");
        assert_eq!(part.model, "");
        Ok(())
    }

    #[tokio::test]
    async fn into_partid_reuses_existing() -> TbResult<()> {
        let (mut store, mut session) = setup();
        session.queue("/gear/b1", &gear_json("b1", Some(0)));
        let first = into_partid("b1".to_string(), &mut session, &mut store).await?;
        let second = into_partid("b1".to_string(), &mut session, &mut store).await?;
        assert_eq!(first, second);
        assert_eq!(session.requests, vec!["/gear/b1".to_string()]);
        Ok(())
    }

    #[tokio::test]
    async fn into_partid_propagates_error() {
        let (mut store, mut session) = setup();
        session.queue_error("/gear/b1", Error::BadRequest("nope".into()));
        let res = into_partid("b1".to_string(), &mut session, &mut store).await;
        assert!(matches!(res, Err(Error::AnyFailure(_))));
    }

    async fn seed_part(
        store: &mut TestStravaStore,
        session: &mut TestStravaSession,
        source: &str,
    ) -> TbResult<PartId> {
        session.queue(&format!("/gear/{source}"), &gear_json(source, Some(0)));
        into_partid(source.to_string(), session, store).await
    }

    #[tokio::test]
    async fn gear_strava_url_bike() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let part_id = seed_part(&mut store, &mut session, "b42").await?;
        let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
        assert_eq!(url, "https://strava.com/bikes/42");
        Ok(())
    }

    #[tokio::test]
    async fn gear_strava_url_shoes() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let part_id = seed_part(&mut store, &mut session, "g1").await?;
        let url = strava_url(i32::from(part_id), &mut session, &mut store).await?;
        assert_eq!(url, "https://www.strava.com/settings/gear");
        Ok(())
    }

    #[tokio::test]
    async fn gear_strava_url_unknown_source() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let part_id = seed_part(&mut store, &mut session, "x9").await?;
        let res = strava_url(i32::from(part_id), &mut session, &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
        Ok(())
    }

    #[tokio::test]
    async fn gear_strava_url_missing_source() {
        let (mut store, mut session) = setup();
        let part = Part::create(
            "Local Part".into(),
            "Brand".into(),
            "Model".into(),
            1.into(),
            None,
            time::OffsetDateTime::now_utc(),
            String::new(),
            &session,
            &mut store,
        )
        .await
        .unwrap();
        let res = strava_url(i32::from(part.id), &mut session, &mut store).await;
        assert!(matches!(res, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn gear_strava_url_forbidden_other_user() -> TbResult<()> {
        let (mut store, mut session) = setup();
        let part_id = seed_part(&mut store, &mut session, "b42").await?;
        let mut other = TestStravaSession::new(UserId::from(2), 43.into());
        let res = strava_url(i32::from(part_id), &mut other, &mut store).await;
        assert!(matches!(res, Err(Error::Forbidden(_))));
        Ok(())
    }
}
