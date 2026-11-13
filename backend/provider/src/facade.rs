//! The provider's door into the domain (spec #781).
//!
//! [`EntityFacade`] is the trait a provider goes through to reach the domain:
//! the operations a provider drives, each a one-line forward to the concrete
//! domain op, generic over the store. A provider function is generic over
//! `D: EntityFacade` (static dispatch); tests swap in the
//! [`MockEntityFacade`] (under `test-support`).
//!
//! [`MockEntityFacade`]: crate::test_support::MockEntityFacade

#![allow(clippy::too_many_arguments)]
use time::OffsetDateTime;

use tb_domain::{
    Activity, ActivityId, Notification, NotificationKind, NotificationLifecycle,
    NotificationSubject, Part, PartId, PartTypeId, Session, Stat, Store, Summary, TbResult, UserId,
};

/// The provider's door into the domain (spec #781): the operations a provider
/// drives, each a one-line forward to the concrete domain op, generic over the
/// store.
///
/// Dispatch is static — a provider function is generic over
/// `D: EntityFacade`. Under `test-support`, `mockall`'s `#[automock]`
/// generates [`MockEntityFacade`]; tests set expectations with the store type
/// as a turbofish, e.g. `expect_upsert_activity::<MyStore>()`.
///
/// [`MockEntityFacade`]: crate::test_support::MockEntityFacade
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[async_trait::async_trait]
pub trait EntityFacade<C: Send + Sync>: Send + Sync {
    // ── Activity ─────────────────────────────────────────────────────────
    /// Upsert the activity and register its usage (the domain's
    /// `Activity::upsert`).
    async fn upsert_activity(
        &self,
        activity: Activity,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Summary>;

    /// Read the activity (the domain's `ActivityId::read` — the ownership
    /// check is the read).
    async fn read_activity(
        &self,
        id: &ActivityId,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Activity>;

    /// Delete the activity and unregister its usage (the domain's
    /// `ActivityId::delete`).
    async fn delete_activity(
        &self,
        id: ActivityId,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Summary>;

    // ── Part ─────────────────────────────────────────────────────────────
    /// Create a part (the domain's `Part::create`).
    async fn create_part(
        &self,
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Part>;

    /// Look up the part carrying the given source (the domain's
    /// `Part::by_source`).
    async fn part_by_source(&self, source: &str, store: &mut C) -> TbResult<Option<Part>>;

    /// Read the part (the domain's `PartId::part` — the ownership check is
    /// the read).
    async fn part(&self, id: PartId, user: Box<dyn Session>, store: &mut C) -> TbResult<Part>;

    // ── User ─────────────────────────────────────────────────────────────
    /// Create a user (the domain's `UserId::create`). `provider` is the
    /// activity provider the user connected with (`'strava'` | `'icu'`), set
    /// at signup and immutable (spec #763).
    async fn create_user(
        &self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        provider: &str,
        store: &mut C,
    ) -> TbResult<UserId>;

    /// Update a user (the domain's `UserId::update`).
    async fn update_user(
        &self,
        id: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        store: &mut C,
    ) -> TbResult<UserId>;

    /// Read the user's stat (the domain's `UserId::get_stat`).
    async fn get_stat(&self, id: &UserId, store: &mut C) -> TbResult<Stat>;

    /// Read the user's admin flag (the domain's `UserId::is_admin`).
    async fn user_admin(&self, id: &UserId, store: &mut C) -> TbResult<bool>;

    /// Read the user's provider identity (the domain's `UserId::provider`).
    async fn user_provider(&self, id: &UserId, store: &mut C) -> TbResult<String>;

    /// Delete the user plus everything they own (the domain's
    /// `UserId::delete`).
    async fn delete_user(&self, id: &UserId, store: &mut C) -> TbResult<()>;

    // ── Notification ─────────────────────────────────────────────────────
    /// Create a notification (the domain's `Notification::create`): the
    /// store assigns the id and timestamps it. The provider's only door to
    /// mint a notification (e.g. the icu stub-skip notice, spec #764).
    async fn create_notification(
        &self,
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
        store: &mut C,
    ) -> TbResult<Notification>;
}

/// The concrete [`EntityFacade`]: a stateless unit struct that forwards each
/// operation to the corresponding domain op.
#[derive(Debug, Default, Clone, Copy)]
pub struct EntityFacadeImpl;

#[async_trait::async_trait]
impl<C: Store + Sync + 'static> EntityFacade<C> for EntityFacadeImpl {
    async fn upsert_activity(
        &self,
        activity: Activity,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Summary> {
        activity.upsert(&*user, store).await
    }

    async fn read_activity(
        &self,
        id: &ActivityId,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Activity> {
        id.read(&*user, store).await
    }

    async fn delete_activity(
        &self,
        id: ActivityId,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Summary> {
        id.delete(&*user, store).await
    }

    async fn create_part(
        &self,
        name: String,
        vendor: String,
        model: String,
        what: PartTypeId,
        source: Option<String>,
        purchase: OffsetDateTime,
        user: Box<dyn Session>,
        store: &mut C,
    ) -> TbResult<Part> {
        Part::create(name, vendor, model, what, source, purchase, &*user, store).await
    }

    async fn part_by_source(&self, source: &str, store: &mut C) -> TbResult<Option<Part>> {
        Part::by_source(source, store).await
    }

    async fn part(&self, id: PartId, user: Box<dyn Session>, store: &mut C) -> TbResult<Part> {
        id.part(&*user, store).await
    }

    async fn create_user(
        &self,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        provider: &str,
        store: &mut C,
    ) -> TbResult<UserId> {
        UserId::create(firstname, lastname, avatar, provider, store).await
    }

    async fn update_user(
        &self,
        id: &UserId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        store: &mut C,
    ) -> TbResult<UserId> {
        id.update(firstname, lastname, avatar, store).await
    }

    async fn get_stat(&self, id: &UserId, store: &mut C) -> TbResult<Stat> {
        id.get_stat(store).await
    }

    async fn user_admin(&self, id: &UserId, store: &mut C) -> TbResult<bool> {
        id.is_admin(store).await
    }

    async fn user_provider(&self, id: &UserId, store: &mut C) -> TbResult<String> {
        id.provider(store).await
    }

    async fn delete_user(&self, id: &UserId, store: &mut C) -> TbResult<()> {
        id.delete(store).await
    }

    async fn create_notification(
        &self,
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
        store: &mut C,
    ) -> TbResult<Notification> {
        Notification::create(recipient, lifecycle, kind, subject, message, store).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tb_domain::test_support::{MockStore, mock_session};
    use tb_domain::{ActTypeId, OnboardingStatus, User};

    /// A canned [`User`] for `id` (inert defaults for the rest).
    fn user(id: UserId) -> User {
        User {
            id,
            name: "Test User".to_string(),
            firstname: "Test".to_string(),
            avatar: None,
            is_admin: false,
            onboarding_status: OnboardingStatus::Completed,
            provider: "strava".to_string(),
        }
    }

    /// A canned [`Activity`] for `id` owned by `user` (inert defaults for the rest).
    fn activity(id: ActivityId, user: UserId) -> Activity {
        Activity {
            id,
            user_id: user,
            what: ActTypeId::from(1),
            name: "Test Ride".to_string(),
            start: OffsetDateTime::UNIX_EPOCH,
            duration: 3600,
            time: None,
            distance: None,
            climb: None,
            descend: None,
            energy: None,
            gear: None,
            device_name: None,
            external_id: None,
        }
    }

    #[tokio::test]
    async fn part_by_source_forwards_to_domain() -> TbResult<()> {
        let mut store = MockStore::new();
        store.expect_partid_get_by_source().returning(|_s| Ok(None));
        let facade = EntityFacadeImpl;
        let part = facade.part_by_source("b1", &mut store).await?;
        assert!(part.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn create_user_forwards_to_domain() -> TbResult<()> {
        let mut store = MockStore::new();
        store
            .expect_user_create()
            .returning(|_f, _l, _a, _p| Ok(user(UserId::from(42))));
        let facade = EntityFacadeImpl;
        let id = facade
            .create_user("first", "last", &None, "strava", &mut store)
            .await?;
        assert_eq!(id, UserId::from(42));
        Ok(())
    }

    #[tokio::test]
    async fn read_activity_forwards_to_domain() -> TbResult<()> {
        let mut store = MockStore::new();
        let act = activity(ActivityId::from("strava:1"), UserId::from(1));
        let act_read = act.clone();
        store
            .expect_activity_read_by_id()
            .returning(move |_id| Ok(Some(act_read.clone())));
        let facade = EntityFacadeImpl;
        let session: Box<dyn Session> = Box::new(mock_session(UserId::from(1)));
        let act = facade
            .read_activity(&ActivityId::from("strava:1"), session, &mut store)
            .await?;
        assert_eq!(act.id, ActivityId::from("strava:1"));
        Ok(())
    }

    /// The `#[automock]` witness is the seam a provider test mocks: set an
    /// expectation and the canned outcome is served without touching the
    /// domain. The store type `C` is inferred from the argument.
    #[tokio::test]
    async fn mock_domain_facade_serves_canned_outcome() -> TbResult<()> {
        let mut mock = MockEntityFacade::new();
        mock.expect_part_by_source()
            .returning(|_source, _store| Ok(None));
        let mut store = MockStore::new();
        let part = mock.part_by_source("b1", &mut store).await?;
        assert!(part.is_none());
        Ok(())
    }
}
