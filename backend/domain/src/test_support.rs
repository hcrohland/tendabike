// backend/domain/src/test_support.rs

pub mod fixtures;

pub use crate::snapshot::StoreSnapshot;

mod mock_store;
pub use mock_store::MockStore;

// --- Test constants for use in unit tests ---

/// PartTypeId constants for tests (UPPERCASE per Rust naming conventions).
/// These replace the constants that were previously defined in PartTypeId impl block.
pub mod part_type_ids {
    use crate::PartTypeId;

    pub const CHAIN: PartTypeId = PartTypeId::from_id(4);
    pub const BIKE: PartTypeId = PartTypeId::from_id(1);
    pub const REAR_WHEEL: PartTypeId = PartTypeId::from_id(5);
    pub const CASSETTE: PartTypeId = PartTypeId::from_id(9);
    pub const SEATPOST: PartTypeId = PartTypeId::from_id(10);
    pub const SADDLE: PartTypeId = PartTypeId::from_id(11);
    pub const DERAILLEUR: PartTypeId = PartTypeId::from_id(12);
    pub const CRANK: PartTypeId = PartTypeId::from_id(13);
    pub const CHAINRING: PartTypeId = PartTypeId::from_id(14);
    pub const BRAKE_ROTOR: PartTypeId = PartTypeId::from_id(15);
    pub const FORK: PartTypeId = PartTypeId::from_id(16);
    pub const REAR_SHOCK: PartTypeId = PartTypeId::from_id(17);
    pub const HANDLEBAR: PartTypeId = PartTypeId::from_id(18);
    pub const BOTTOM_BRACKET: PartTypeId = PartTypeId::from_id(19);
    pub const HEADSET: PartTypeId = PartTypeId::from_id(20);
    pub const FRONT_WHEEL: PartTypeId = PartTypeId::from_id(2);
    pub const TIRE: PartTypeId = PartTypeId::from_id(3);
    pub const BRAKE_PAD: PartTypeId = PartTypeId::from_id(6);
    pub const FRONT_BRAKE: PartTypeId = PartTypeId::from_id(7);
    pub const REAR_BRAKE: PartTypeId = PartTypeId::from_id(8);
}

// --- Summary assertions ---

/// An [`IdKeyedMap`] holding exactly one entry: the tombstone for `id`.
/// For whole-map assertions: `assert_eq!(summary.parts, only_tombstone(id))`.
pub fn only_tombstone<K, E>(id: K) -> IdKeyedMap<K, E>
where
    K: Eq + Hash,
{
    let mut map = IdKeyedMap::default();
    map.tombstone(id);
    map
}

/// The sole live entity in `map`, cloned out; panics if `map` holds
/// anything other than exactly one live entry. For tests asserting an op
/// reported exactly one entity: `let svc = sole_live(&summary.services)`.
pub fn sole_live<K, E>(map: &IdKeyedMap<K, E>) -> E
where
    E: Clone,
{
    let mut live = map.live();
    assert_eq!(
        live.len(),
        1,
        "expected exactly one live entry, found {}",
        live.len()
    );
    live.pop().expect("one live entry, asserted above")
}

use std::hash::Hash;

use crate::*;

/// A [`MockSession`] for `user`, `shop`, and `admin`, whose `check_owner`
/// behaves like the trait default (delegating to `user_id`). The
/// expectations accept any call count (including zero), so a session built
/// for one op need not predict which of its methods the op calls.
fn build_session(user: UserId, shop: Option<ShopId>, admin: bool) -> MockSession {
    let mut mock = MockSession::new();
    mock.expect_user_id().times(0..).return_const(user);
    mock.expect_shop().times(0..).return_const(shop);
    mock.expect_is_admin().times(0..).return_const(admin);
    mock.expect_check_owner()
        .times(0..)
        .returning(move |owner, error| user.check_owner(owner, error));
    mock
}

/// A customer [`MockSession`] for `user` (no shop, not admin).
pub fn mock_session(user: UserId) -> MockSession {
    build_session(user, None, false)
}

/// A shop-owner [`MockSession`] for `user` acting in `shop` (not admin).
pub fn mock_session_with_shop(user: UserId, shop: ShopId) -> MockSession {
    build_session(user, Some(shop), false)
}

/// An admin [`MockSession`] for `user` (no shop).
pub fn mock_session_admin(user: UserId) -> MockSession {
    build_session(user, None, true)
}
