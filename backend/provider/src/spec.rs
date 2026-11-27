//! The provider marker (spec #1029, ticket #1030): the type-only carrier of a
//! provider's wire shapes.
//!
//! [`ProviderSpec`] is the marker a provider driver declares: its associated
//! types (the user row, the event row, the owner id, the event key, the
//! driver type), the provider's enum value as a constant (the marker → enum
//! direction), the type-level constructor for the sync event (how this event
//! type represents a sync), and the identity accessors (the event's
//! id/owner/key, the user row's tbid/owner) as type-level functions.
//!
//! It is plumbing, not an interface: the author implements it to declare their
//! wire shapes, and the kit's generic traits ([`Store`](crate::store::Store))
//! are generic over it. The per-provider data-store traits are re-expressed
//! once per provider as the [`Store`](crate::store::Store) trait (the database
//! crate implements the thin forwards).

use std::fmt::{Debug, Display};

use crate::Provider;
use crate::facade::UserId;

/// The provider marker (spec #1029): the type-only carrier of a provider's
/// wire shapes.
///
/// A provider driver declares its wire shapes by implementing [`ProviderSpec`]:
/// the user row type, the event row type, the owner id type (the queue's
/// partition key), the event's dedup key type, and the driver type (the unit
/// struct that implements the full
/// [`Provider`](crate::driver::Provider) driver trait — the marker's
/// `Driver` is the driver's single home, spec #1062).
/// The marker also carries the provider's enum value as a constant (the
/// marker → enum direction), the type-level constructor for the sync event
/// (how this event type represents a sync), and the event-identity accessors
/// (id, owner, key) as type-level functions.
///
/// This is plumbing, not an interface: the author implements it to declare
/// their wire shapes, and the kit's generic traits are generic over it. The
/// per-provider data-store traits are re-expressed once per provider as the
/// [`Store`](crate::store::Store) trait (the database crate implements the thin
/// forwards); the event-identity trait's accessors live here as the marker's
/// type-level functions.
pub trait ProviderSpec: Send + Sync + Sized {
    /// The provider's enum value (the marker → enum direction).
    const PROVIDER: Provider;

    /// The provider's user row (the `strava_users` / `icu_users` row).
    type User: Clone + Debug + Send + Sync + 'static;

    /// The provider's event row (the `strava_events` / `icu_events` row).
    type Event: Clone + Debug + Send + Sync + 'static;

    /// The owner id (the queue's partition key — the `owner_id` /
    /// `icu_athlete_id` column).
    type Owner: Clone + Debug + Send + Sync + Display + 'static;

    /// The event's dedup key (the object the event refers to — the Strava
    /// `object_id`, the icu `activity_id`). Only the latest event per key is
    /// interesting.
    type Key: Clone + Debug + Send + Sync + 'static;

    /// The driver type: the unit struct that implements the full
    /// [`Provider`](crate::driver::Provider) driver trait (spec #1029; the
    /// bound consolidated from the narrow token-refresh trait in spec
    /// #1062). The marker's `Driver` is the driver's single home: the kit's
    /// generic token machinery reaches the driver's token hooks through
    /// this bound, and the lane drives the marker's driver.
    type Driver: crate::driver::Provider<Self>;

    /// The type-level constructor for the sync event: how this event type
    /// represents a sync (the durable backfill cursor). `owner` is the
    /// provider's owner id and `time` is the sync's time (the Strava
    /// `event_time`, the icu `oldest` bound).
    fn make_sync(owner: Self::Owner, time: i64) -> Self::Event;

    /// The event row id (`None` until the row is stored).
    fn event_id(e: &Self::Event) -> Option<i32>;

    /// The event's owner (the queue's partition key).
    fn event_owner(e: &Self::Event) -> &Self::Owner;

    /// The event's dedup key.
    fn event_key(e: &Self::Event) -> &Self::Key;

    /// The user row's TendaBike user id (the domain-side identity the shared
    /// skeleton drives through the facade).
    fn user_tbid(user: &Self::User) -> &UserId;

    /// The user row's owner (the provider id — the queue's partition key).
    fn user_owner(user: &Self::User) -> &Self::Owner;
}
