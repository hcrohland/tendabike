use crate::{
    Notification, NotificationId, NotificationKind, NotificationLifecycle, NotificationSubject,
    TbResult, UserId,
};

#[async_trait::async_trait]
/// A trait representing a store for `Notification` objects.
pub trait NotificationStore {
    /// Creates a new `Notification`.
    ///
    /// Used by domain operations as a side effect — never called directly by
    /// the user (issue #586).
    async fn notification_create(
        &mut self,
        recipient: UserId,
        lifecycle: NotificationLifecycle,
        kind: NotificationKind,
        subject: NotificationSubject,
        message: Option<String>,
    ) -> TbResult<Notification>;

    /// Reads a notification by id, applying its lifecycle:
    /// - **Persistent**: marks it `seen` and returns the updated row.
    /// - **Transient**: deletes the row and returns the notification as it
    ///   was before deletion.
    ///
    /// Returns a `NotFound` error when no notification with that id exists.
    async fn notification_read(&mut self, id: NotificationId) -> TbResult<Notification>;

    /// Reads all of a user's notifications, applying each one's lifecycle:
    /// - **Persistent**: marks them `seen` and returns the updated rows.
    /// - **Transient**: deletes the rows and returns the notifications as
    ///   they were before deletion.
    async fn notification_read_all(&mut self, recipient: UserId) -> TbResult<Vec<Notification>>;

    /// Gets all of a user's notifications without applying any lifecycle
    /// (pure read, for hydration).
    async fn notification_all_for_user(&mut self, recipient: UserId)
    -> TbResult<Vec<Notification>>;
}
