use diesel_derive_newtype::*;
use newtype_derive::*;
use serde_derive::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::*;
use schema::services;

#[derive(
    DieselNewType, Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default,
)]
pub struct ServiceId(Uuid);

NewtypeDisplay! { () pub struct ServiceId(); }
NewtypeFrom! { () pub struct ServiceId(Uuid); }

/// Timeline of service
///
#[derive(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Queryable,
    Identifiable,
    Insertable,
    AsChangeset,
)]
pub struct Service {
    id: ServiceId,
    /// the part serviced
    part_id: PartId,
    /// when it was serviced
    #[serde(with = "time::serde::rfc3339")]
    time: OffsetDateTime,
    /// when there was a new service
    #[serde(with = "time::serde::rfc3339")]
    redone: OffsetDateTime,
    // we do not accept theses values from the client!
    name: String,
    notes: String,
    usage: UsageId,
}
