//! The intervals.icu gear import (spec #765): the wire shape of an
//! intervals.icu gear, its type mapping, and the import of the user's gear
//! list (`GET /athlete/{id}/gear`) into the domain's parts.
//!
//! The `icu:` [`Part.source`] prefix is an icu concern: the domain's
//! `Part.source` is an opaque string (ADR-0013), so the prefix construction
//! lives here, private to the icu module.

use time::OffsetDateTime;

use super::*;
use crate::EntityFacade;
use tb_domain::{PartId, PartTypeId, TbResult, UserId};

/// The intervals.icu wire shape of a gear (`GET /athlete/{id}/gear`).
///
/// The intervals.icu API returns more fields than the domain models
/// (`distance`, `retired`); only `id` (→ the namespaced `Part.source`) and
/// `name` (→ `Part.name`) are copied in [`into_partid`]. Vendor and model are
/// left empty (the API has no vendor/model split), and `distance`/`retired`
/// are ignored (parsed, not modeled in the domain).
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct IcuGear {
    pub id: String,
    pub name: String,
    /// The gear type (e.g. `Bike`, `Trainer`, `Shoes`).
    #[serde(rename = "type")]
    pub type_: String,
    /// The gear's accumulated distance (ignored — not modeled in the domain).
    #[allow(dead_code)]
    pub distance: Option<f64>,
    /// Whether the gear is retired (ignored — not modeled in the domain).
    #[allow(dead_code)]
    pub retired: Option<bool>,
}

impl IcuGear {
    /// The part type for this gear's icu type: `Bike`/`Trainer` → 1, `Shoes`
    /// → 301; `None` for unsupported types (the gear is skipped on import).
    fn what(&self) -> Option<PartTypeId> {
        Some(
            match self.type_.as_str() {
                "Bike" | "Trainer" => 1,
                "Shoes" => 301,
                _ => return None,
            }
            .into(),
        )
    }
}

/// Maps an icu gear to a Tendabike [`PartId`]: reuses the part carrying the
/// namespaced `icu:{id}` source, or creates it (vendor and model left empty).
/// `None` for unsupported types (the gear is skipped).
pub(crate) async fn into_partid<D: EntityFacade<S>, S: IcuDataStore>(
    gear: &IcuGear,
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<PartId>> {
    let source = format!("icu:{}", gear.id);
    if let Some(part) = facade.part_by_source(&source, store).await? {
        return Ok(Some(part.id));
    }
    let Some(what) = gear.what() else {
        return Ok(None);
    };
    debug!("New Gear");
    let name = gear.name.clone();
    let vendor = String::new();
    let model = String::new();
    let purchase = OffsetDateTime::now_utc();
    let session = Box::new(OnBehalf(user_id));
    let part = facade
        .create_part(
            name,
            vendor,
            model,
            what,
            Some(source),
            purchase,
            session,
            store,
        )
        .await?;
    Ok(Some(part.id))
}

/// Imports the user's icu gear (the hydration write's provider side, spec
/// #765): fetches `GET /athlete/{id}/gear` and imports each supported gear.
/// Returns the created part ids.
///
/// Skips the icu call silently when the rate limiter reports no budget:
/// hydration is best-effort and must not be blocked by a non-essential API
/// call (mirrors the Strava driver's `update_gear`).
pub async fn update_gear<D: EntityFacade<S>, S: IcuDataStore>(
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Vec<PartId>> {
    if super::ratelimit::limited_until().is_some() {
        log::debug!("update_gear: rate limited, skipping hydration");
        return Ok(Vec::new());
    }
    let icu_id = super::event::get_icu_id(user_id, store).await?;
    let gears: Vec<IcuGear> =
        super::http::request_json(user_id, &format!("/athlete/{icu_id}/gear"), store).await?;
    let mut parts = Vec::new();
    for gear in gears {
        if let Some(part_id) = into_partid(&gear, user_id, facade, store).await? {
            parts.push(part_id);
        }
    }
    Ok(parts)
}

#[cfg(test)]
mod gear_tests;
