//! The identity-only acting session ([`OnBehalf`]) every driver's event lane
//! shares.

use crate::facade::{Session, ShopId, UserId};

/// A minimal, identity-only [`Session`] for the event lane (ADR-0011): the
/// event lane has no acting session, so where it crosses into a session-typed
/// domain call it constructs this adapter. Personal, non-admin, no shop —
/// the event lane's actual context (ADR-0009).
#[derive(Debug, Clone, Copy)]
pub(crate) struct OnBehalf(UserId);

impl OnBehalf {
    /// Construct the identity-only session for `user_id`: no active shop, not
    /// an admin — the event lane's context.
    pub(crate) fn new(user_id: UserId) -> Self {
        Self(user_id)
    }
}

impl Session for OnBehalf {
    fn user_id(&self) -> UserId {
        self.0
    }
    fn shop(&self) -> Option<ShopId> {
        None
    }
    fn is_admin(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identity-only session carries the user's id and the event lane's
    /// context: no active shop, not an admin.
    #[test]
    fn on_behalf_carries_the_identity_and_the_event_lane_context() {
        let session = OnBehalf::new(UserId::from(7));
        assert_eq!(session.user_id(), UserId::from(7));
        assert_eq!(session.shop(), None);
        assert!(!session.is_admin());
    }
}
