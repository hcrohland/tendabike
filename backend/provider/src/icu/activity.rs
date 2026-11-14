//! intervals.icu activity import (spec #764): the wire shape of an
//! intervals.icu activity, its mapping to the domain [`Activity`], the
//! activity-type mapping ([`get_type`]), stub detection, and the gear
//! reference resolution.
//!
//! The `icu:` [`ActivityId`] prefix is an icu concern: the domain's
//! `ActivityId` is an opaque string (ADR-0013), so the prefix knowledge
//! lives in the crate-level `activity_id` module (the `ProviderActivityId`
//! the opaque id converts to and from).

use time::OffsetDateTime;

use super::*;
use crate::{EntityFacade, IcuActivityId, ProviderActivityId};
use tb_domain::{
    ActTypeId, Activity, Error, Notification, NotificationKind, NotificationLifecycle,
    NotificationSubject, PartId, Summary, UserId,
};

/// The intervals.icu URL for an activity (the `/source/activities/{id}`
/// redirect target): the driver extracts the raw id from the domain's opaque
/// [`ActivityId`], verifies ownership via the facade, then builds the
/// provider URL from the raw id.
pub async fn activity_url<D, S>(
    id: ActivityId,
    user: Box<dyn Session>,
    facade: &D,
    store: &mut S,
) -> TbResult<String>
where
    D: EntityFacade<S>,
    S: Send + Sync,
{
    let ProviderActivityId::Icu(raw) = ProviderActivityId::try_from(id)? else {
        return Err(Error::BadRequest("not an icu activity".to_string()));
    };
    let full: ActivityId = ProviderActivityId::Icu(raw.clone()).into();
    facade.read_activity(&full, user, store).await?;
    Ok(format!("https://intervals.icu/activities/{raw}"))
}

/// A gear reference carried by an icu activity.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct IcuGearRef {
    id: String,
}

/// The intervals.icu wire shape of an activity (`GET /activity/{id}`).
///
/// The intervals.icu API returns more fields than the domain models
/// (calories, heart rates, watts); only the fields with a domain counterpart
/// are copied in [`IcuActivity::into_activity`]. `energy` is the activity's
/// joules (`icu_joules`), mirroring the Strava driver's `kilojoules` →
/// `energy`.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct IcuActivity {
    pub id: String,
    /// The activity type (e.g. `Ride`, `MountainBikeRide`).
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// Covered distance, in meters.
    pub distance: Option<f64>,
    /// Moving time, in seconds.
    pub moving_time: Option<i32>,
    /// Elapsed (total) time, in seconds.
    pub elapsed_time: i32,
    /// Total climbing, in meters.
    pub total_elevation_gain: Option<f64>,
    /// Total descent, in meters (→ the domain's `descend`).
    pub total_elevation_loss: Option<f64>,
    /// The gear the activity used (its id is namespaced as `icu:<id>`).
    pub gear: Option<IcuGearRef>,
    /// Energy output, in joules (→ the domain's `energy`).
    pub icu_joules: Option<f64>,
}

impl IcuActivity {
    /// Whether the activity is a Strava-sourced stub: both `distance` and
    /// `moving_time` are null-or-zero (spec #764). Such an activity carries no
    /// usable data and is skipped on import.
    fn is_stub(&self) -> bool {
        self.distance.unwrap_or(0.0) <= 0.0 && self.moving_time.unwrap_or(0) <= 0
    }

    /// Maps the icu activity to the domain [`Activity`].
    async fn into_activity<D: EntityFacade<S>, S: IcuDataStore>(
        self,
        user_id: UserId,
        facade: &D,
        store: &mut S,
    ) -> TbResult<Activity> {
        let IcuActivity {
            id,
            type_,
            name,
            start_date,
            distance,
            moving_time,
            elapsed_time,
            total_elevation_gain,
            total_elevation_loss,
            gear,
            icu_joules,
        } = self;
        let what = Self::get_type(&type_)?;
        let gear = match gear {
            Some(g) => gear_into_partid(&g.id, facade, store).await?,
            None => None,
        };
        Ok(Activity {
            id: ProviderActivityId::Icu(IcuActivityId::from(id)).into(),
            what,
            gear,
            user_id,
            name,
            start: start_date,
            duration: elapsed_time,
            time: moving_time,
            distance: distance.map(|d| d.round() as i32),
            climb: total_elevation_gain.map(|c| c.round() as i32),
            descend: total_elevation_loss.map(|d| d.round() as i32),
            energy: icu_joules.map(|e| e.round() as i32),
            device_name: None,
            external_id: None,
        })
    }

    /// Maps intervals.icu activity type strings to Tendabike types: the
    /// common wire-name arms from the provider kit's table
    /// ([`crate::activity_type::common_type`]), plus the cycling-subtype arms
    /// that are icu-specific — `MountainBikeRide`/`EMountainBikeRide`→11,
    /// `GravelRide`→12, `Cyclocross`→13, and `TrackRide`→0. Unknown types are
    /// `Err(BadRequest)`.
    fn get_type(t: &str) -> TbResult<ActTypeId> {
        if let Some(id) = crate::activity_type::common_type(t) {
            return Ok(id);
        }
        Ok(match t {
            "MountainBikeRide" | "EMountainBikeRide" => 11,
            "GravelRide" => 12,
            "Cyclocross" => 13,
            "TrackRide" => 0,
            _ => return Err(Error::BadRequest(format!("unsupported activity {t}"))),
        }
        .into())
    }
}

/// Resolves an activity's icu gear reference to a domain [`PartId`]: a
/// `part_by_source` lookup for the namespaced `icu:<id>` source. `None` when
/// the gear part has not been imported yet (the activity's gear reference is
/// dropped — the gear `Part` itself imports via the gear endpoint, a later
/// ticket).
async fn gear_into_partid<D: EntityFacade<S>, S: IcuDataStore>(
    id: &str,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<PartId>> {
    let source = format!("icu:{id}");
    Ok(facade.part_by_source(&source, store).await?.map(|p| p.id))
}

/// Fetches the full icu activity and imports it into the domain (spec #764).
///
/// Returns `Ok(None)` when the activity is a stub (skipped — the gear
/// reference is dropped and no upsert happens), or `Ok(Some(summary))` with
/// Imports an icu activity that is already in hand (no HTTP fetch): stub
/// check → map → upsert. Used by the backfill (the list response carries all
/// the fields the domain records).
pub(crate) async fn import_activity_data<D: EntityFacade<S>, S: IcuDataStore>(
    act: IcuActivity,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<Summary>> {
    if act.is_stub() {
        debug!("skipping stub icu activity {}", act.id);
        return Ok(None);
    }
    let activity = act.into_activity(user_id, facade, store).await?;
    let session = Box::new(OnBehalf(user_id));
    Ok(Some(
        facade.upsert_activity(activity, session, store).await?,
    ))
}

/// the upsert's `Summary` for a real activity. Errors (a 401, a 429, a DB
/// failure) bubble up for the caller to classify.
pub(crate) async fn import_activity<D: EntityFacade<S>, S: IcuDataStore>(
    activity_id: &str,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<Summary>> {
    let act: IcuActivity =
        super::http::request_json(user_id, &format!("/activity/{activity_id}"), store).await?;
    if act.is_stub() {
        debug!("skipping stub icu activity {activity_id}");
        return Ok(None);
    }
    let activity = act.into_activity(user_id, facade, store).await?;
    let session = Box::new(OnBehalf(user_id));
    Ok(Some(
        facade.upsert_activity(activity, session, store).await?,
    ))
}

/// The one transient notification a stub-skip batch carries (spec #764): the
/// `message` is the skip count. A stub batch (≥1 skipped) creates exactly one.
pub(crate) async fn stub_notice<D: EntityFacade<S>, S: IcuDataStore>(
    user_id: UserId,
    skipped: usize,
    facade: &D,
    store: &mut S,
) -> TbResult<Notification> {
    facade
        .create_notification(
            user_id,
            NotificationLifecycle::Transient,
            NotificationKind::StubsSkipped,
            NotificationSubject::StubsSkipped,
            Some(skipped.to_string()),
            store,
        )
        .await
}

#[cfg(test)]
mod activity_tests;
