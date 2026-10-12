// backend/domain/src/test_support.rs
//
//! Store-agnostic test support for the domain suite (spec #510): the
//! in-memory store itself is `tb_teststore::TestStore`, taken as a
//! dev-dependency by the integration tests in `tests/`. What stays here
//! is what names no store: the session double, the part-type constants,
//! and the summary assertion helpers.

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

use std::hash::Hash;

use crate::{IdKeyedMap, Session, ShopId, TbResult, UserId};

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

// --- Session ---

/// Test session for attachment tests
pub struct TestSession {
    user_id: UserId,
    shop: Option<ShopId>,
    admin: bool,
}

impl TestSession {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            shop: None,
            admin: false,
        }
    }

    pub fn with_shop(user_id: UserId, shop: ShopId) -> Self {
        Self {
            user_id,
            shop: Some(shop),
            admin: false,
        }
    }

    pub fn with_admin(user_id: UserId, admin: bool) -> Self {
        Self {
            user_id,
            shop: None,
            admin,
        }
    }
}

impl Session for TestSession {
    fn user_id(&self) -> UserId {
        self.user_id
    }
    fn shop(&self) -> Option<ShopId> {
        self.shop
    }
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        Ok(())
    }
    fn is_admin(&self) -> bool {
        self.admin
    }

    // Unused parameters in check_owner use the default implementation
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        self.user_id.check_owner(owner, error)
    }
}
