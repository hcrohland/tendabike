/*
This file defines the Strava provider marker.

`StravaSpec` is the type carrier for Strava's wire shapes (spec #1029) — the
[`StravaUser`] row, the [`Event`] row, the [`StravaId`] owner, and the `i64`
dedup key (the `object_id`). The store is the single uniform
[`Store`](crate::store::Store) interface, generic over this marker (the
database crate implements it once per provider).
*/

use super::{StravaId, StravaUser, event::Event};
use crate::Provider;
use crate::ProviderSpec;
use crate::facade::UserId;

/// The Strava provider marker (spec #1029): the type carrier for Strava's wire
/// shapes — the [`StravaUser`] row, the [`Event`] row, the [`StravaId`] owner,
/// the `i64` dedup key (the `object_id`).
pub struct StravaSpec;

impl ProviderSpec for StravaSpec {
    const PROVIDER: Provider = Provider::Strava;
    type User = StravaUser;
    type Event = Event;
    type Owner = StravaId;
    type Key = i64;
    fn make_sync(owner: StravaId, time: i64) -> Event {
        Event {
            id: None,
            object_type: super::event::ObjectType::Sync,
            object_id: 0,
            owner_id: owner,
            event_time: time,
            ..Default::default()
        }
    }
    fn event_id(e: &Event) -> Option<i32> {
        e.id
    }
    fn event_owner(e: &Event) -> &StravaId {
        &e.owner_id
    }
    fn event_key(e: &Event) -> &i64 {
        &e.object_id
    }
    fn user_tbid(user: &StravaUser) -> &UserId {
        &user.tendabike_id
    }
    fn user_owner(user: &StravaUser) -> &StravaId {
        &user.id
    }
}

#[cfg(test)]
mod tests {
    use super::super::event::ObjectType;
    use super::*;

    /// The marker's `PROVIDER` constant is the enum value (the marker → enum
    /// direction).
    #[test]
    fn strava_spec_provider_constant() {
        assert_eq!(StravaSpec::PROVIDER, Provider::Strava);
    }

    /// The marker's `make_sync` builds the durable sync row: a `Sync` for the
    /// owner at `time`, `object_id` 0, no row id yet.
    #[test]
    fn strava_spec_make_sync_builds_sync_row() {
        let owner = StravaId::from(42);
        let event = StravaSpec::make_sync(owner, 1_700_000_000);
        assert_eq!(event.object_type, ObjectType::Sync);
        assert_eq!(event.object_id, 0);
        assert_eq!(event.owner_id, owner);
        assert_eq!(event.event_time, 1_700_000_000);
        assert_eq!(event.id, None);
    }

    /// The marker's event-identity accessors read the row's id, owner, and key.
    #[test]
    fn strava_spec_event_identity_accessors() {
        let event = StravaSpec::make_sync(StravaId::from(42), 1_700_000_000);
        assert_eq!(StravaSpec::event_id(&event), None);
        assert_eq!(*StravaSpec::event_owner(&event), StravaId::from(42));
        assert_eq!(*StravaSpec::event_key(&event), 0);
    }
}
