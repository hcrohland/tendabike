/*
This file defines the intervals.icu provider marker.

`IcuSpec` is the type carrier for icu's wire shapes (spec #1029) — the
[`IcuUser`] row, the [`IcuEvent`] row, the [`IcuId`] owner, and the `String`
dedup key (the `activity_id`). The per-provider data-store trait is gone: the
store is the single uniform [`Store`](crate::store::Store) interface, generic
over this marker (the database crate implements it once per provider).
*/

use super::{IcuEvent, IcuId, IcuUser};
use crate::Provider;
use crate::ProviderSpec;
use crate::facade::UserId;

/// The icu provider marker (spec #1029): the type carrier for icu's wire
/// shapes — the [`IcuUser`] row, the [`IcuEvent`] row, the [`IcuId`] owner,
/// the `String` dedup key (the `activity_id`), and the
/// [`IcuDriver`](super::driver::IcuDriver) ([`ProviderSpec::Driver`]). The
/// driver uses the refresh hooks' default no-ops: intervals.icu tokens are
/// durable (no cache, no refresh).
pub struct IcuSpec;

impl ProviderSpec for IcuSpec {
    const PROVIDER: Provider = Provider::Icu;
    type User = IcuUser;
    type Event = IcuEvent;
    type Owner = IcuId;
    type Key = String;
    type Driver = super::driver::IcuDriver;
    fn make_sync(owner: IcuId, time: i64) -> IcuEvent {
        IcuEvent::new_sync(owner, time)
    }
    fn event_id(e: &IcuEvent) -> Option<i32> {
        e.id
    }
    fn event_owner(e: &IcuEvent) -> &IcuId {
        &e.icu_athlete_id
    }
    fn event_key(e: &IcuEvent) -> &String {
        &e.activity_id
    }
    fn user_tbid(user: &IcuUser) -> &UserId {
        &user.tendabike_id
    }
    fn user_owner(user: &IcuUser) -> &IcuId {
        &user.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icu::IcuObjectType;

    /// The marker's `PROVIDER` constant is the enum value (the marker → enum
    /// direction).
    #[test]
    fn icu_spec_provider_constant() {
        assert_eq!(IcuSpec::PROVIDER, Provider::Icu);
    }

    /// The marker's `make_sync` builds the durable sync row: a `Sync` for the
    /// owner with the `oldest` bound, the `"sync"` sentinel key, the `0`
    /// `newest` sentinel, no row id yet.
    #[test]
    fn icu_spec_make_sync_builds_sync_row() {
        let owner = IcuId::from("ath-42");
        let event = IcuSpec::make_sync(owner.clone(), 1_700_000_000);
        assert_eq!(event.object_type, IcuObjectType::Sync);
        assert_eq!(event.icu_athlete_id, owner);
        assert_eq!(event.activity_id, "sync");
        assert_eq!(event.event_time, 0);
        assert_eq!(event.oldest, Some(1_700_000_000));
        assert_eq!(event.id, None);
    }

    /// The marker's event-identity accessors read the row's id, owner, and key.
    #[test]
    fn icu_spec_event_identity_accessors() {
        let event = IcuSpec::make_sync(IcuId::from("ath-42"), 1_700_000_000);
        assert_eq!(IcuSpec::event_id(&event), None);
        assert_eq!(*IcuSpec::event_owner(&event), IcuId::from("ath-42"));
        assert_eq!(*IcuSpec::event_key(&event), "sync");
    }
}
